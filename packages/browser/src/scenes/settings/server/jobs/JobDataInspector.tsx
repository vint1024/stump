import { Preformatted, Sheet, usePrevious } from '@stump/components'
import { FragmentType, graphql, useFragment } from '@stump/graphql'
import { useLocaleContext } from '@stump/i18n'

const fragment = graphql(`
	fragment JobDataInspector on CoreJobOutput {
		__typename
		... on LibraryScanOutput {
			totalFiles
			totalDirectories
			ignoredFiles
			skippedFiles
			ignoredDirectories
			createdMedia
			updatedMedia
			createdSeries
			updatedSeries
		}
		... on SeriesScanOutput {
			totalFiles
			ignoredFiles
			skippedFiles
			createdMedia
			updatedMedia
		}
		... on ThumbnailGenerationOutput {
			visitedFiles
			skippedFiles
			generatedThumbnails
			removedThumbnails
		}
	}
`)

export type JobDataInspectorFragment = FragmentType<typeof fragment>

type Props = {
	data?: JobDataInspectorFragment | null
	onClose: () => void
}

export default function JobDataInspector({ data, onClose }: Props) {
	const { t } = useLocaleContext()
	const inlineData = useFragment(fragment, data)
	const fallback = usePrevious(data)
	const displayedData = inlineData || fallback

	return (
		<Sheet
			open={!!data}
			onClose={onClose}
			title={t('scenes.settings.server.jobs.JobDataInspector.title')}
			description={t('scenes.settings.server.jobs.JobDataInspector.description')}
		>
			<Preformatted
				title={t('scenes.settings.server.jobs.JobDataInspector.rawJson')}
				content={displayedData}
			/>
		</Sheet>
	)
}
