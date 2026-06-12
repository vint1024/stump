import { useGraphQLMutation, useSDK, useSuspenseGraphQL } from '@stump/client'
import {
	Alert,
	AlertDescription,
	Button,
	ConfirmationModal,
	Heading,
	Text,
} from '@stump/components'
import { graphql, MetadataResetImpact, UserPermission } from '@stump/graphql'
import { useLocaleContext } from '@stump/i18n'
import { useQueryClient } from '@tanstack/react-query'
import { Construction } from 'lucide-react'
import { Suspense, useCallback, useEffect, useState } from 'react'
import { useNavigate } from 'react-router'
import { toast } from 'sonner'

import { SceneContainer } from '@/components/container'
import { ResetMetadata } from '@/components/metadata/metadataEditor'
import { SeriesMetadataEditor } from '@/components/series/metadata'
import { useAppContext } from '@/context'
import paths from '@/paths'

import { useSeriesContext } from '../../context'
import MergeSeriesSection from './MergeSeriesSection'
import SeriesTagEditor from './SeriesTagEditor'
import SeriesThumbnailSelector from './SeriesThumbnailSelector'

const query = graphql(`
	query SeriesSettingsScene($id: ID!) {
		seriesById(id: $id) {
			id
			...SeriesThumbnailSelector
			library {
				id
			}
			tags {
				id
				name
			}
			metadata {
				...SeriesMetadataEditor
			}
		}
	}
`)

const analyzeMutation = graphql(`
	mutation SeriesSettingsSceneAnalyze($id: ID!) {
		analyzeSeries(id: $id)
	}
`)

const resetMetadataMutation = graphql(`
	mutation SeriesSettingsSceneResetMetadata($id: ID!, $impact: MetadataResetImpact!) {
		resetSeriesMetadata(id: $id, impact: $impact) {
			id
		}
	}
`)

const regenerateThumbnailMutation = graphql(`
	mutation SeriesSettingsSceneRegenerateThumbnail($id: ID!, $forceRegenerate: Boolean!) {
		generateSeriesThumbnail(id: $id, forceRegenerate: $forceRegenerate)
	}
`)

const deleteSeriesMutation = graphql(`
	mutation SeriesSettingsSceneDeleteSeries($id: ID!) {
		deleteSeries(id: $id)
	}
`)

export default function SeriesSettingsScene() {
	const { t } = useLocaleContext()
	const { sdk } = useSDK()
	const { series } = useSeriesContext()
	const { checkPermission } = useAppContext()
	const client = useQueryClient()

	const navigate = useNavigate()
	const [showDeleteConfirm, setShowDeleteConfirm] = useState(false)

	const {
		data: { seriesById },
	} = useSuspenseGraphQL(query, sdk.cacheKey('seriesById', [series.id, 'settings']), {
		id: series.id ?? '',
	})

	const { data, mutate: analyze, isPending } = useGraphQLMutation(analyzeMutation)
	const { mutate: resetMetadata } = useGraphQLMutation(resetMetadataMutation)
	const { mutate: regenerateThumbnail, isPending: isRegeneratingThumbnail } = useGraphQLMutation(
		regenerateThumbnailMutation,
		{
			onSuccess: () => {
				toast.success(t('seriesSettingsScene.thumbnail.toasts.started'))
			},
			onError: (error) => {
				console.error('Failed to regenerate series thumbnail', error)
				toast.error(t('seriesSettingsScene.thumbnail.toasts.failed'))
			},
		},
	)

	const handleAnalyze = useCallback(() => analyze({ id: series.id }), [analyze, series.id])
	const handleResetMetadata = useCallback(
		(impact: MetadataResetImpact) => resetMetadata({ id: series.id, impact }),
		[resetMetadata, series.id],
	)
	const handleRegenerateThumbnail = useCallback(
		() => regenerateThumbnail({ id: series.id, forceRegenerate: true }),
		[regenerateThumbnail, series.id],
	)

	const { mutate: deleteSeries, isPending: isDeleting } = useGraphQLMutation(deleteSeriesMutation, {
		onSuccess: () => {
			toast.success(t('seriesSettingsScene.delete.toasts.deleted'))
			setShowDeleteConfirm(false)
			client.invalidateQueries({ queryKey: [sdk.cacheKeys.series] })
			client.invalidateQueries({ queryKey: [sdk.cacheKeys.libraries] })
			const libraryId = seriesById?.library?.id
			navigate(libraryId ? paths.librarySeries(libraryId) : paths.libraries())
		},
		onError: (error) => {
			console.error('Failed to delete series', error)
			toast.error(t('seriesSettingsScene.delete.toasts.failed'))
		},
	})

	useEffect(() => {
		if (!seriesById) {
			navigate(paths.notFound())
		}
	}, [seriesById, navigate])

	if (!seriesById) {
		return null
	}

	return (
		<SceneContainer>
			<div className="gap-y-6 flex flex-col items-start text-left">
				<Alert variant="warning">
					<Construction />
					<AlertDescription>{t('seriesSettingsScene.developmentAlert')}</AlertDescription>
				</Alert>

				<div className="gap-y-2 flex flex-col">
					<div>
						<Heading size="sm">{t('seriesSettingsScene.analysis.heading')}</Heading>
						<Text size="sm" variant="muted">
							{t('seriesSettingsScene.analysis.description')}
						</Text>
					</div>

					<div>
						<Button
							title={
								data
									? t('seriesSettingsScene.analysis.inProgress')
									: t('seriesSettingsScene.analysis.buttonTitle')
							}
							size="md"
							variant="primary"
							onClick={handleAnalyze}
							disabled={!!data || isPending}
						>
							{t('seriesSettingsScene.analysis.button')}
						</Button>
					</div>
				</div>

				{checkPermission(UserPermission.EditMetadata) && (
					<Suspense>
						<SeriesTagEditor seriesId={seriesById.id} tags={seriesById.tags} />
					</Suspense>
				)}

				<div className="gap-y-2 flex flex-col">
					<div>
						<Heading size="sm">{t('seriesSettingsScene.thumbnail.heading')}</Heading>
						<Text size="sm" variant="muted">
							{t('seriesSettingsScene.thumbnail.description')}
						</Text>
					</div>

					<div className="gap-3 flex flex-col items-start">
						<SeriesThumbnailSelector fragment={seriesById} />
						{checkPermission(UserPermission.EditThumbnails) && (
							<Button
								title={t('seriesSettingsScene.thumbnail.regenerateTitle')}
								size="md"
								variant="outline"
								onClick={handleRegenerateThumbnail}
								disabled={isRegeneratingThumbnail}
							>
								{t('seriesSettingsScene.thumbnail.regenerate')}
							</Button>
						)}
					</div>
				</div>

				{checkPermission(UserPermission.ManageLibrary) && (
					<Suspense>
						<MergeSeriesSection seriesId={seriesById.id} />
					</Suspense>
				)}

				<div className="gap-y-2 flex w-full flex-col">
					<div className="flex items-end justify-between">
						<div>
							<Heading size="sm">{t('seriesSettingsScene.metadata.heading')}</Heading>
							<Text variant="muted">{t('seriesSettingsScene.metadata.description')}</Text>
						</div>
						{checkPermission(UserPermission.EditMetadata) && (
							<ResetMetadata onConfirmReset={handleResetMetadata} />
						)}
					</div>
					<SeriesMetadataEditor seriesId={seriesById.id} data={seriesById.metadata} />
				</div>

				{checkPermission(UserPermission.ManageLibrary) && (
					<div className="gap-y-2 flex w-full flex-col">
						<div>
							<Heading size="sm">{t('seriesSettingsScene.delete.heading')}</Heading>
							<Text size="sm" variant="muted">
								{t('seriesSettingsScene.delete.description')}
							</Text>
						</div>
						<div className="flex">
							<Button
								type="button"
								variant="danger"
								disabled={isDeleting}
								onClick={() => setShowDeleteConfirm(true)}
							>
								{t('seriesSettingsScene.delete.button')}
							</Button>
						</div>
						<ConfirmationModal
							title={t('seriesSettingsScene.delete.confirm.title')}
							description={t('seriesSettingsScene.delete.confirm.description')}
							confirmText={t('seriesSettingsScene.delete.confirm.confirm')}
							confirmVariant="danger"
							isOpen={showDeleteConfirm}
							onClose={() => setShowDeleteConfirm(false)}
							onConfirm={() => deleteSeries({ id: series.id })}
							trigger={null}
						/>
					</div>
				)}
			</div>
		</SceneContainer>
	)
}
