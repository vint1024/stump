use std::{
	collections::{HashMap, HashSet},
	num::NonZeroU16,
};

use models::entity::{media, media_metadata, media_tag, tag};
use quick_xml::{
	escape::unescape,
	events::{BytesText, Event},
	Reader, Writer,
};
use sea_orm::ConnectionTrait;

use crate::{
	filesystem::media::{process_metadata_raw_async, ProcessedMediaMetadata},
	CoreError,
};

const SHELL_COMIC_INFO: &str = r#"<?xml version="1.0"?>
<ComicInfo xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema">
</ComicInfo>
"#;

// 1. assume fetched information from database and pass as inputs to function call (i.e., purely functional)
// 2. give function reference to book, let it query and make detemrinations, etc (i.e., handle the damn thing)
//
// general steps are:
pub async fn write_comic_info<C>(conn: &C, book_id: String) -> Result<(), CoreError>
where
	C: ConnectionTrait,
{
	// 1. load book with metadata
	// select * from media where id = :book_id left join media_metadata on media_id = book_id ~ rough
	// ^ media::Model + media_metadata::Model -> ModelWithMetadata { media, metadata }
	let media::ModelWithMetadata { media, metadata } =
		media::ModelWithMetadata::find_by_id(book_id.clone())
			.into_model::<media::ModelWithMetadata>()
			.one(conn)
			.await?
			.ok_or(CoreError::NotFound(format!(
				"Book with ID {book_id} not found"
			)))?;

	let Some(metadata) = metadata else {
		return Err(CoreError::InternalError(
			"This book does not have any existing metadata to writeback".to_string(),
		));
	};

	let existing_tags = tag::Entity::find_for_media_id(&media.id)
		.all(conn)
		.await?
		.into_iter()
		.map(|tag| tag.name)
		.collect();

	// 2. ensure file exists
	let file_path = media.path.clone();
	if let Err(e) = tokio::fs::metadata(&file_path).await {
		tracing::error!(error = ?e, "Could not find book on disk");
		return Err(e.into());
	}

	// 3. pull the raw metadata from file to be used in step 4
	// we collect the CURRENT metadata so that we can inline replace tags manually to minimize risk of
	// data loss which could happen if we were to just dump our rust struct back into XML (e.g., if
	// we didn't process X field used by some software, we don't want to lose it)
	let existing_xml_bytes = process_metadata_raw_async(file_path)
		.await?
		.unwrap_or_else(|| SHELL_COMIC_INFO.to_string().into_bytes().to_vec());
	let xml_string = String::from_utf8_lossy(&existing_xml_bytes).to_string();

	// 4. take db metadata and merge into existing xml (or template, if none exist)
	let updated_metadata = merge_metadata_into_xml(metadata, existing_tags, xml_string);

	// 5. TODO: write updated_metadata xml as ComicInfo.xml in file, see TODO remark in process.rs re: trait fns
	// 6. Done, wow how ez

	unimplemented!()
}

// i edit metadata -> i tell stump to save the file -> server calls core to save file
// 1. server loads current metadata -> pass file location + metadata to core
// 2. just call core -> core load metadata and handle

type XmlString = String;

// aim for suboptimal first:
// 1. loop through each tag
// 2. find corresponding tag in `metadata`
// 3. if it is a processable tag (e.g., Title) then replace value <Title>value</Title>
// ^ tricky part with this is reconciling the ones we could not find (e.g., what if we used shell)
// emphasis on suboptimal:
// - track the fields visited? e.g. SUPPORTED_FIELDS = ["title", "writers", ..., etc]
// - if by end of loop, we filter unvisited and append to end of xml (inside <ComicInfo> ofc)

// !! pseudo code !!
//
// loop:
// if event is start: <-- we can get the tag name (e.g., "Title")
//      current_tag = event.tag
//      visited_tags["Title"] = true
// if event is text: <-- this is the value inside the tag (e.g., "The Way of Kings")
//      existing_text = event.text
//      new_text = metadata["Title"] <-- pull from database metadata
//      el.replace_text(existing_text, new_text)
// if event is end:
//      current_tag = None
// if event is eof:
//      break
//
// for unprocessed_field in visited_tags.filter(value is false):
//      write_field(metadata[field])
//
// !! pseudo code !!
//
// ^ open questions:
// 1. where are we writing to? do we:
//      - maintain a writer and sync position with reader?
//      - easiest is probably to just have a writer attached to a separate buffer that
//        always writes for each event, not just text replacement
// 2. actual api for reader/writer, not pseudo code

fn merge_metadata_into_xml(
	metadata: media_metadata::Model,
	existing_tags: Vec<String>,
	existing_xml: XmlString,
) -> Result<XmlString, CoreError> {
	let mut reader = Reader::from_str(&existing_xml);
	reader.config_mut().trim_text(true);

	let mut writer = Writer::new(Vec::new());
	let mut buf = Vec::new();
	let mut current_tag = String::default();
	let mut visited = HashSet::<String>::new();

	let metadata_map = build_metadata_map(metadata.clone(), existing_tags.clone());

	loop {
		match reader.read_event_into(&mut buf) {
			Ok(Event::Start(e)) => {
				let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
				current_tag = tag.clone();

				if let Some(None) = metadata_map.get(tag.as_str()) {
					visited.insert(tag.clone());
				} else {
					writer.write_event(Event::Start(e))?;
				}
			},
			// TODO: determine if Event::Text would pop off if e.g. <Title></Title> and if so
			// how do we handle it here
			Ok(Event::Text(e)) => match metadata_map.get(current_tag.as_str()) {
				Some(Some(value)) => {
					// TODO: do we need to escape value?
					writer.write_event(Event::Text(BytesText::new(value)))?;
					visited.insert(current_tag.clone());
				},
				Some(None) => {
					// already handled in Event::Start
				},
				None => {
					tracing::debug!(
						?current_tag,
						"This field is not managed by Stump. Writing it back as-is."
					);
					writer.write_event(Event::Text(e))?;
				},
			},
			// TODO: how do we handle <Title /> OR <Title></Title> if that ends up here
			Ok(Event::Empty(e)) => {
				unimplemented!()
			},
			Ok(Event::End(e)) => {
				// not end, i.e. eof, just end of tag
				let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();

				if let Some(None) = metadata_map.get(tag.as_str()) {
					current_tag.clear();
				} else {
					// if it is ComicInfo tag ^ ->
					//          writer write it:
					//              1. start event for the tag
					//              2. text like before (same todo as above re: do we need to escape)
					//              3. end event for the tag
					// TODO: do this
					writer.write_event(Event::End(e))?;
				}
			},
			Ok(Event::Eof) => break,
			// if none of the above, idk how to handle it so just write it as-is
			Ok(e) => {
				tracing::debug!(
					event = ?e,
					"Ecountered event which is not explicitly handled. Writing back as-is"
				);
				writer.write_event(e)?;
			},
			_ => {
				unimplemented!()
			},
		}
	}

	Ok(String::from_utf8(writer.into_inner())?)
}

fn build_metadata_map(
	metadata: media_metadata::Model,
	existing_tags: Vec<String>,
) -> HashMap<&'static str, Option<String>> {
	let mut map = HashMap::new();

	map.insert("Title", metadata.title.clone());
	map.insert("Series", metadata.series.clone());

	// TODO: add all the other fckn fields

	map
}

#[cfg(test)]
mod tests {
	use super::*;
	use models::entity::media_metadata;
	use quick_xml::de::from_str as xml_from_str;

	#[test]
	fn test_basic_conversion() {
		// this is what currently exists in the embedded ComicInfo.xml
		let existing_xml = r#"<?xml version="1.0"?>
<ComicInfo xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema">
    <Title>Invincible 001</Title>
</ComicInfo>
"#;

		let metadata = media_metadata::Model {
			title: Some("Invincible #1".to_string()),
			..Default::default()
		};

		let xml_string =
			merge_metadata_into_xml(metadata.clone(), vec![], existing_xml.to_string())
				.expect("Should have converted comic info");

		assert!(xml_string.contains("Invincible #1")); // the updated title

		let deserialized_xml: ProcessedMediaMetadata =
			xml_from_str(&xml_string).expect("Should have properly parsed xml_string");

		assert_eq!(metadata.title, deserialized_xml.title);
	}

	#[test]
	fn test_full_conversion() {
		// TODO: add tags to this example
		let existing_xml = r#"<?xml version="1.0"?>
<ComicInfo xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <Title>Heist Part Two</Title>
  <Series>The Amazing Spider-Man</Series>
  <Number>9</Number>
  <Volume>5</Volume>
  <Summary>There's been a major theft the likes of which we've never seen and for once, The Black Cat didn't do it.

But Spider-Man might need the help of his once-foe-once-friend-once-crime-boss Felicia Hardy, the Black Cat!

*List of covers and their creators:*
 Cover | Name                                                | Creator(s)                                | Sidebar Location |
-------------------------------------------------------------------------------------------------------------------------
 Reg   | Regular Cover                                       | Humberto Ramos &amp; Edgar Delgado            | 1                |
 Var   | Uncanny X-Men Variant Cover                         | Clayton Crain                             | 8                |
 Var   | Variant Cover                                       | Mike Wieringo, Tim Townsend &amp; Jason Keith | 2                |
 Var   | Virgin Variant Cover                                | Mike Wieringo, Tim Townsend &amp; Jason Keith | 9-10             |
 RE    | ComicXposure Variant Cover                          | Jeff Dekal                                | 3                |
 RE    | ComicXposure Virgin Variant Cover                   | Jeff Dekal                                | 4                |
 RE    | Unknown Comic Books Connecting Variant Cover        | Mico Suayan                               | 5                |
 RE    | Unknown Comic Books Connecting Virgin Variant Cover | Mico Suayan                               | 6                |
 2nd   | Second Printing Variant Cover                       | Humberto Ramos                            | 7                |
Note: Unknown Comic Books variant by Mico Suayan connects with Venom #8, Web of Venom: Carnage Born #1, and Amazing Spider-Man #10.</Summary>
  <Notes>Tagged with the ninjas.walk.alone fork of ComicTagger 1.3.4 using info from Comic Vine on 2022-02-09 10:38:41.  [Issue ID 692056]</Notes>
  <Year>2019</Year>
  <Month>01</Month>
  <Day>01</Day>
  <Writer>Nick Spencer</Writer>
  <Penciller>Humberto Ramos, Michele Bandini</Penciller>
  <Inker>Michele Bandini, Victor Olazaba</Inker>
  <Colorist>Edgar Delgado, Erick Arciniega</Colorist>
  <Letterer>Joe Caramagna</Letterer>
  <CoverArtist>Clayton Crain, Edgar Delgado, Humberto Ramos, Jason Keith, Jeff Dekal, Mike Wieringo, Tim Townsend</CoverArtist>
  <Editor>Kathleen Wisneski, Nick Lowe</Editor>
  <Publisher>Marvel</Publisher>
  <Web>https://comicvine.gamespot.com/the-amazing-spider-man-9-heist-part-two/4000-692056/</Web>
  <PageCount>24</PageCount>
  <Characters>Armadillo, Black Cat, Captain America, Carlie Cooper, Hawkeye, Human Torch, Iron Man, Jarvis, Kate Bishop, Mary Jane, Odessa Drake, Spider-Man, Squirrel Girl, Thing, Thor, Walter Hardy, Wasp</Characters>
  <Teams>Thieves Guild</Teams>
  <Pages>
						<Page Image="0" Type="FrontCover" ImageSize="741291" />
						<Page Image="1" ImageSize="1383958" />
						<Page Image="2" ImageSize="1897301" />
						<Page Image="3" ImageSize="1956592" />
						<Page Image="4" ImageSize="1348064" />
						<Page Image="5" ImageSize="2050915" />
						<Page Image="6" ImageSize="1570591" />
						<Page Image="7" ImageSize="1868853" />
						<Page Image="8" ImageSize="1703356" />
						<Page Image="9" ImageSize="1835994" />
						<Page Image="10" ImageSize="1600821" />
						<Page Image="11" ImageSize="2207163" />
						<Page Image="12" ImageSize="2401548" />
						<Page Image="13" ImageSize="1891242" />
						<Page Image="14" ImageSize="2012601" />
						<Page Image="15" ImageSize="1761542" />
						<Page Image="16" ImageSize="1797303" />
						<Page Image="17" ImageSize="2332849" />
						<Page Image="18" ImageSize="2023561" />
						<Page Image="19" ImageSize="2239700" />
						<Page Image="20" ImageSize="2271632" />
						<Page Image="21" ImageSize="1879935" />
						<Page Image="22" ImageSize="1663028" />
						<Page Image="23" ImageSize="750766" />
  </Pages>
</ComicInfo>"#;

		let metadata = media_metadata::Model {
			// TODO: add all the fields
			..Default::default()
		};

		let xml_string =
			merge_metadata_into_xml(metadata.clone(), vec![], existing_xml.to_string())
				.expect("Should have converted comic info");

		let deserialized_xml: ProcessedMediaMetadata =
			xml_from_str(&xml_string).expect("Should have properly parsed xml_string");

		// TODO: assert each field in deserialized_xml
	}
}

// LOGAN REFERENCES:
// - https://anansi-project.github.io/docs/comicinfo/documentation
//   - we don't currenly deserialize all fields, so manual xml reader/writer has
//     benefit of finding tags and inline replacing instead of
//     relying on serde to dump the entire thing which could technically
//     lead to data loss since we do not intake or represent all which is supported
