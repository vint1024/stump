import { cn, Text } from '@stump/components'
import { SmartListGroupedItem, SmartListViewColumn } from '@stump/graphql'
import { ColumnDef, createColumnHelper } from '@tanstack/react-table'
import { ChevronDown } from 'lucide-react'

type EntityGroup = SmartListGroupedItem
const columnHelper = createColumnHelper<EntityGroup>()

type TranslateFn = (key: string, options?: Record<string, unknown>) => string
const NS = 'scenes.smartList.items.table.groupColumns'
/**
 * A fallback translate function for non-rendering callers (e.g. the store, which only
 * reads column ids and never renders header/cell text). It returns the key as-is.
 */
const identityT: TranslateFn = (key) => key

const buildNameColumn = (isGroupedBySeries: boolean, t: TranslateFn) =>
	columnHelper.accessor('entity.name', {
		cell: ({
			row: {
				original: {
					entity: { name },
				},
				getToggleExpandedHandler,
				getIsExpanded,
				getCanExpand,
			},
		}) => {
			const isExpanded = getIsExpanded()

			return (
				<button
					title={isExpanded ? t(`${NS}.collapse`) : t(`${NS}.expand`)}
					className="gap-x-1 flex items-center"
					onClick={getToggleExpandedHandler()}
					disabled={!getCanExpand()}
				>
					<ChevronDown
						className={cn(
							'h-4 w-4 shrink-0 text-muted-foreground transition-transform duration-200',
							{
								'rotate-180': isExpanded,
							},
						)}
					/>
					<Text className="text-sm md:text-base line-clamp-1 text-left">{name}</Text>
				</button>
			)
		},
		enableGlobalFilter: true,
		enableSorting: true,
		header: ({ table: { getToggleAllRowsExpandedHandler, getIsAllRowsExpanded } }) => {
			const isAllRowsExpanded = getIsAllRowsExpanded()

			return (
				<div className="gap-x-1 flex items-center">
					<button
						onClick={(e) => {
							// Don't update the sorting state when clicking the expand all button
							e.stopPropagation()
							const handler = getToggleAllRowsExpandedHandler()
							handler(e)
						}}
						title={isAllRowsExpanded ? t(`${NS}.collapseAll`) : t(`${NS}.expandAll`)}
					>
						<ChevronDown
							className={cn('h-4 w-4 text-muted-foreground transition-transform duration-200', {
								'rotate-180': isAllRowsExpanded,
							})}
						/>
					</button>
					<Text className="text-sm" variant="muted">
						{isGroupedBySeries ? t(`${NS}.series`) : t(`${NS}.library`)}
					</Text>
				</div>
			)
		},
		id: 'name',
	})

const buildBooksCountColumn = (t: TranslateFn) =>
	columnHelper.accessor(({ books }) => books.length, {
		cell: ({
			row: {
				original: { books },
			},
		}) => (
			<Text size="sm" variant="muted">
				{books.length}
			</Text>
		),
		enableGlobalFilter: true,
		enableSorting: true,
		header: () => (
			<Text size="sm" className="text-left" variant="muted">
				{t(`${NS}.books`)}
			</Text>
		),
		id: 'books',
	})

export const getColumnMap = (isGroupedBySeries: boolean, t: TranslateFn = identityT) =>
	({
		books: buildBooksCountColumn(t),
		name: buildNameColumn(isGroupedBySeries, t),
	}) as Record<string, ColumnDef<EntityGroup>>

export const getColumnOptionMap = (isGroupedBySeries: boolean, t: TranslateFn = identityT) =>
	({
		name: t(`${NS}.nameOption`, {
			scope: isGroupedBySeries ? t(`${NS}.seriesScope`) : t(`${NS}.libraryScope`),
		}),
		books: t(`${NS}.books`),
	}) as Record<string, string>

export const buildDefaultColumns = (isGroupedBySeries: boolean, t: TranslateFn = identityT) =>
	[buildNameColumn(isGroupedBySeries, t), buildBooksCountColumn(t)] as ColumnDef<EntityGroup>[]

export const buildColumns = (
	isGroupedBySeries: boolean,
	columns?: SmartListViewColumn[],
	t: TranslateFn = identityT,
) => {
	if (!columns?.length) {
		return buildDefaultColumns(isGroupedBySeries, t)
	}

	const sortedColumns = columns.sort((a, b) => a.position - b.position)
	const selectedColumnIds = sortedColumns.map(({ id }) => id)

	const columnMap = getColumnMap(isGroupedBySeries, t)

	return selectedColumnIds.map((id) => columnMap[id]).filter(Boolean) as ColumnDef<EntityGroup>[]
}
