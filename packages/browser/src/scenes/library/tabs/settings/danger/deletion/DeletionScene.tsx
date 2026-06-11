import CleanLibrary from './CleanLibrary'
import DeleteLibrary from './DeleteLibrary'
import MetadataWriteback from './MetadataWriteback'

export default function DeletionScene() {
	return (
		<div className="gap-12 flex flex-col">
			<MetadataWriteback />
			<CleanLibrary />
			<DeleteLibrary />
		</div>
	)
}
