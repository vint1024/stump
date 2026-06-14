import { useGraphQLMutation, useInfiniteCursorGraphQL } from '@stump/client'
import { Avatar, Card, Text } from '@stump/components'
import { BookClubMembersTableQuery, graphql } from '@stump/graphql'
import { useLocaleContext } from '@stump/i18n'
import { BookClubMemberRoleSpec } from '@stump/sdk'
import { ColumnDef, createColumnHelper } from '@tanstack/react-table'
import upperFirst from 'lodash/upperFirst'
import { useMemo, useState } from 'react'
import { toast } from 'sonner'

import { Table } from '@/components/table'
import { useAppContext } from '@/context'

import { useBookClubManagement } from '../context'
import MemberActionMenu from './MemberActionMenu'
import RemoveMemberConfirmation from './RemoveMemberConfirmation'

const query = graphql(`
	query BookClubMembersTable($id: ID!, $pagination: CursorPagination!) {
		bookClubMembers(bookClubId: $id, pagination: $pagination) {
			nodes {
				id
				avatarUrl
				isCreator
				displayName
				role
				userId
			}
			cursorInfo {
				nextCursor
				limit
			}
		}
	}
`)

const removeMutation = graphql(`
	mutation RemoveBookClubMember($bookClubId: ID!, $memberId: ID!) {
		removeBookClubMember(bookClubId: $bookClubId, memberId: $memberId) {
			id
		}
	}
`)

export default function MembersTable() {
	const { t } = useLocaleContext()
	const { user } = useAppContext()
	const {
		club: { id, roleSpec },
	} = useBookClubManagement()

	const { data, hasNextPage, isFetchingNextPage, fetchNextPage, refetch } =
		useInfiniteCursorGraphQL(query, ['bookClubMembers', id], {
			id,
			pagination: { limit: 50 },
		})
	const members = useMemo(
		() => data?.pages.flatMap((page) => page.bookClubMembers.nodes) ?? [],
		[data],
	)

	const [removingMember, setRemovingMember] = useState<Member | null>(null)

	const { mutate: removeMember } = useGraphQLMutation(removeMutation, {
		onSuccess: () => refetch(),
		onError: (error) => {
			console.error('Error removing member:', error)
			toast.error(t('scenes.bookClub.tabs.settings.members.MembersTable.removeError'))
		},
	})

	const columns = useMemo(
		() => [
			...createBaseColumns(roleSpec, t),
			columnHelper.display({
				id: 'actions',
				cell: ({ row: { original } }) => {
					if (original.userId === user?.id || original.isCreator) {
						return null
					}

					return <MemberActionMenu onSelectForRemoval={() => setRemovingMember(original)} />
				},
			}),
		],
		[roleSpec, user, t],
	)

	return (
		<>
			<RemoveMemberConfirmation
				isOpen={!!removingMember}
				onClose={(didConfirm) => {
					if (didConfirm && removingMember) {
						removeMember({
							bookClubId: id,
							memberId: removingMember.id,
						})
					}
					setRemovingMember(null)
				}}
			/>
			<Card>
				<Table
					sortable
					columns={columns}
					options={{
						// Cursor-paginated via the "Load more" button below — render every
						// loaded member on a single page so the table's offset pager (which
						// can't drive a forward-only cursor) stays inert/disabled.
						manualPagination: true,
						pageCount: 1,
						state: {
							columnPinning: {
								right: ['actions'],
							},
							pagination: { pageIndex: 0, pageSize: members.length || 10 },
						},
					}}
					data={members ?? []}
					fullWidth
					cellClassName="bg-background"
				/>
				{hasNextPage && (
					<div className="p-2 flex justify-center">
						<button
							type="button"
							disabled={isFetchingNextPage}
							onClick={() => fetchNextPage()}
							className="rounded-sm p-1 outline-none focus-visible:ring-2 focus-visible:ring-brand-400 disabled:opacity-50"
						>
							<Text className="cursor-pointer underline" size="sm" variant="muted">
								{t('scenes.bookClub.tabs.settings.members.MembersTable.loadMore')}
							</Text>
						</button>
					</div>
				)}
			</Card>
		</>
	)
}

type Member = BookClubMembersTableQuery['bookClubMembers']['nodes'][number]

const columnHelper = createColumnHelper<Member>()

const createBaseColumns = (spec: BookClubMemberRoleSpec, t: (key: string) => string) =>
	[
		columnHelper.accessor(({ displayName }) => displayName, {
			cell: ({
				row: {
					original: { avatarUrl, displayName },
				},
			}) => (
				<div className="flex items-center">
					<Avatar className="mr-2" src={avatarUrl ?? undefined} fallback={displayName} />
					<span>{displayName}</span>
				</div>
			),
			header: t('scenes.bookClub.tabs.settings.members.MembersTable.member'),
			id: 'display_name',
		}),
		columnHelper.accessor('role', {
			cell: ({ getValue }) => (
				<span>{spec[getValue()] || upperFirst(getValue().toLowerCase())}</span>
			),
			header: 'Role',
		}),
	] as ColumnDef<Member>[]
