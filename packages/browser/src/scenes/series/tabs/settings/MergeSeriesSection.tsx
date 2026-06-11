import { useGraphQL, useGraphQLMutation, useSDK, useSuspenseGraphQL } from '@stump/client'
import { Button, Heading, Input, NativeSelect, Text } from '@stump/components'
import { graphql } from '@stump/graphql'
import { useLocaleContext } from '@stump/i18n'
import { useQueryClient } from '@tanstack/react-query'
import { useCallback, useEffect, useMemo, useState } from 'react'
import { useDebouncedValue } from 'rooks'
import { toast } from 'sonner'

/** The merge target picker fetches at most this many matches per search */
const CANDIDATE_LIMIT = 50

const LOCALE_BASE = 'seriesSettingsScene.merge'

const query = graphql(`
	query MergeSeriesSection($id: ID!) {
		seriesById(id: $id) {
			id
			library {
				id
			}
			mergedSources {
				name
				path
			}
		}
	}
`)

const candidatesQuery = graphql(`
	query MergeSeriesSectionCandidates($libraryId: String!, $search: String!, $limit: Int!) {
		series(
			filter: { libraryId: { eq: $libraryId }, name: { contains: $search } }
			pagination: { offset: { page: 1, pageSize: $limit } }
		) {
			nodes {
				id
				name: resolvedName
				path
			}
		}
	}
`)

const mergeMutation = graphql(`
	mutation MergeSeriesSectionMerge($targetId: ID!, $sourceIds: [ID!]!) {
		mergeSeries(targetId: $targetId, sourceIds: $sourceIds) {
			id
		}
	}
`)

const unmergeMutation = graphql(`
	mutation MergeSeriesSectionUnmerge($id: ID!) {
		unmergeSeries(id: $id) {
			id
		}
	}
`)

type Props = {
	seriesId: string
}

/**
 * Series settings section for merging this series into another one (its books
 * move over and this folder keeps feeding the target on future scans), and for
 * undoing merges absorbed by this series
 */
export default function MergeSeriesSection({ seriesId }: Props) {
	const { sdk } = useSDK()
	const { t } = useLocaleContext()
	const client = useQueryClient()

	const {
		data: { seriesById },
	} = useSuspenseGraphQL(query, sdk.cacheKey('seriesById', [seriesId, 'merge']), {
		id: seriesId,
	})
	const libraryId = seriesById?.library?.id ?? ''

	// Server-side search instead of fetching every series in the library —
	// large libraries would otherwise pull thousands of rows for a picker
	const [search, setSearch] = useState('')
	const [debouncedSearch] = useDebouncedValue(search, 300)
	const { data: candidatesData } = useGraphQL(
		candidatesQuery,
		sdk.cacheKey('series', [libraryId, 'mergeCandidates', debouncedSearch ?? '']),
		{
			libraryId,
			search: debouncedSearch ?? '',
			limit: CANDIDATE_LIMIT,
		},
	)
	const candidates = useMemo(() => candidatesData?.series.nodes ?? [], [candidatesData])

	const [targetId, setTargetId] = useState<string>('')

	const otherSeries = useMemo(
		() => candidates.filter(({ id }) => id !== seriesId),
		[candidates, seriesId],
	)

	// A narrowed search can drop the picked target from the list — clear the
	// selection rather than keep an invisible one armed behind the button
	useEffect(() => {
		if (targetId && !otherSeries.some(({ id }) => id === targetId)) {
			setTargetId('')
		}
	}, [otherSeries, targetId])
	const mergedSources = seriesById?.mergedSources ?? []

	const invalidate = useCallback(() => {
		client.invalidateQueries({ queryKey: [sdk.cacheKeys.series] })
		client.invalidateQueries({ queryKey: [sdk.cacheKeys.seriesById] })
		client.invalidateQueries({ queryKey: [sdk.cacheKeys.libraries] })
	}, [client, sdk])

	const { mutate: mergeSeries, isPending: isMerging } = useGraphQLMutation(mergeMutation, {
		onSuccess: () => {
			toast.success(t(`${LOCALE_BASE}.toasts.merged`))
			invalidate()
		},
		onError: (error) => {
			console.error('Failed to merge series', error)
			toast.error(t(`${LOCALE_BASE}.toasts.mergeFailed`))
		},
	})

	const { mutate: unmergeSeries, isPending: isUnmerging } = useGraphQLMutation(unmergeMutation, {
		onSuccess: () => {
			toast.success(t(`${LOCALE_BASE}.toasts.restored`))
			invalidate()
		},
		onError: (error) => {
			console.error('Failed to unmerge series', error)
			toast.error(t(`${LOCALE_BASE}.toasts.restoreFailed`))
		},
	})

	const handleMerge = useCallback(() => {
		if (!targetId) return
		mergeSeries({ targetId, sourceIds: [seriesId] })
	}, [mergeSeries, targetId, seriesId])

	return (
		<div className="gap-y-4 flex w-full flex-col">
			<div>
				<Heading size="sm">{t(`${LOCALE_BASE}.heading`)}</Heading>
				<Text size="sm" variant="muted">
					{t(`${LOCALE_BASE}.description`)}
				</Text>
			</div>

			<div className="gap-2 md:max-w-md flex max-w-full flex-col">
				<Input
					placeholder={t(`${LOCALE_BASE}.searchPlaceholder`)}
					value={search}
					onChange={(e) => setSearch(e.target.value)}
				/>
				<div className="gap-2 flex items-center">
					<NativeSelect
						value={targetId}
						onChange={(e) => setTargetId(e.target.value)}
						options={otherSeries.map(({ id, name }) => ({ label: name, value: id }))}
						emptyOption={{ label: t(`${LOCALE_BASE}.selectTarget`), value: '' }}
					/>
					<Button
						type="button"
						variant="outline"
						disabled={!targetId || isMerging}
						onClick={handleMerge}
					>
						{t(`${LOCALE_BASE}.button`)}
					</Button>
				</div>
				{otherSeries.length >= CANDIDATE_LIMIT - 1 && (
					<Text size="xs" variant="muted">
						{t(`${LOCALE_BASE}.limitHint`, { limit: CANDIDATE_LIMIT })}
					</Text>
				)}
			</div>

			{mergedSources.length > 0 && (
				<div className="gap-y-2 flex flex-col">
					<Text size="sm" variant="muted">
						{t(`${LOCALE_BASE}.mergedSources`)}
					</Text>
					<ul className="list-inside list-disc">
						{mergedSources.map(({ name, path }) => (
							<li key={path}>
								<Text size="sm" className="inline">
									{name} <span className="text-foreground-muted">({path})</span>
								</Text>
							</li>
						))}
					</ul>
					<div>
						<Button
							type="button"
							variant="outline"
							disabled={isUnmerging}
							onClick={() => unmergeSeries({ id: seriesId })}
						>
							{t(`${LOCALE_BASE}.unmergeAll`)}
						</Button>
					</div>
				</div>
			)}
		</div>
	)
}
