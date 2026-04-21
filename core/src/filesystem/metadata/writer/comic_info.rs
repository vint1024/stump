use models::entity::{media, media_metadata};
use sea_orm::ConnectionTrait;

use crate::{filesystem::media::ProcessedMediaMetadata, CoreError};

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

	// 2. ensure file exists
	let file_path = media.path.clone();
	if let Err(e) = tokio::fs::metadata(&file_path).await {
		tracing::error!(error = ?e, "Could not find book on disk");
		return Err(e.into());
	}

	// 3. TODO: convert into ComicInfo.xml (e.g., convert_to_comic_info()) <-- use metadata from above
	// 4. TODO: merge/replace with existing WRITE into ZIP
	// 5. Done, wow how ez

	unimplemented!()
}

// i edit metadata -> i tell stump to save the file -> server calls core to save file
// 1. server loads current metadata -> pass file location + metadata to core
// 2. just call core -> core load metadata and handle

type XmlString = String;

fn convert_to_comic_info(
	metadata: media_metadata::Model,
	// TODO: tags?
	// TODO: potentially intake existing as struct?
	existing_xml: ProcessedMediaMetadata,
) -> Result<XmlString, CoreError> {
	/*
	search for: parse_opf_xml --> example API usage:

	   loop {
	   let current_target = None;
		while let Some(event) = reader.read() {
		 match event {
		   Ok(Event::XmlStartTag(e)) => {
			// extract name to set current target
			current_target = Some(e.name);
		   },
		   Ok(Event::Text(e)) => {
		   let Some(target) = current_target else {
			   continue;
		   }
		   // get the text
		   // replace text with metadata

					match target.to_lowercase() {
					"teams" => {
						metada.teams
					}
					}

		   // continue, maybe unset current_target
		   }

		 }
		}

	   }

	*/

	// 1. load and parse into rust struct for us
	// 2. define `apply` to set struct fields from media metadata
	// 3. take sruct and just convert to string
	// 4. write the string

	unimplemented!()
}

// LOGAN REFERENCES:
// - https://serde.rs/
//   - https://serde.rs/json.html
// - https://docs.rs/quick-xml/latest/quick_xml/
// - https://anansi-project.github.io/docs/comicinfo/documentation
//   - we don't currenly deserialize all fields, so manual xml reader/writer has
//     benefit of finding tags and inline replacing instead of
//     relying on serde to dump the entire thing which could technically
//     lead to data loss since we do not intake or represent all which is supported

#[cfg(test)]
mod tests {
	use models::entity::media_metadata;

	use crate::filesystem::{
		media::{metadata_from_buf, ProcessedMediaMetadata},
		metadata::writer::comic_info::convert_to_comic_info,
	};

	// for route 1 (relying on serde + using some unimplement apply function)
	#[test]
	fn test_basic_conversion() {
		// pretend this is what currently exists in ComicInfo.xml
		let basic = ProcessedMediaMetadata {
			title: Some("Invincible 001".to_string()),
			..Default::default()
		};

		// FIXME: won't actually work bc missing fields but fine for demonstration for now
		// let's pretend a user updated the title manually
		let metadata = media_metadata::Model {
			title: Some("Invincible #1".to_string()),
			..Default::default()
		};

		let xml_string = convert_to_comic_info(metadata.clone(), basic.clone())
			.expect("Should have converted comic info");

		let deserialized_xml = metadata_from_buf(&xml_string)
			.expect("Should have properly parsed xml_string");

		assert_eq!(metadata.title, deserialized_xml.title);
	}

	// for route 2 (use reader/writer from quick_xml to create new string)
	#[test]
	fn test_basic_conversion_route_two() {
		// pretend this is what currently exists in ComicInfo.xml
		let basic = quick_xml::se::to_string(&ProcessedMediaMetadata {
			title: Some("Invincible 001".to_string()),
			..Default::default()
		});

		// FIXME: won't actually work bc missing fields but fine for demonstration for now
		// let's pretend a user updated the title manually
		let metadata = media_metadata::Model {
			title: Some("Invincible #1".to_string()),
			..Default::default()
		};

		// let xml_string = update_comic_info(metadata.clone(), basic.clone())
		// 	.expect("Should have updated comic info XML string");

		// let deserialized_xml = metadata_from_buf(&xml_string)
		// 	.expect("Should have properly parsed xml_string");

		// assert_eq!(metadata.title, deserialized_xml.title);
	}
}

/*
<?xml version="1.0"?>
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
</ComicInfo>

 */
