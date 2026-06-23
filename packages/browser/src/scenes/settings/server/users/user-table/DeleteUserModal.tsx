import { useGraphQLMutation, useSDK } from '@stump/client'
import { Button, CheckBox, Dialog } from '@stump/components'
import { graphql } from '@stump/graphql'
import { useLocaleContext } from '@stump/i18n'
import { useQueryClient } from '@tanstack/react-query'
import { useCallback, useState } from 'react'

import { User } from './UserTable'

const mutation = graphql(`
	mutation DeleteUser($id: ID!, $hardDelete: Boolean) {
		deleteUser(id: $id, hardDelete: $hardDelete) {
			id
		}
	}
`)

type Props = {
	deletingUser: User | null
	onClose: () => void
}

const LOCALE_NS = 'scenes.settings.server.users.user-table.DeleteUserModal'

export default function DeleteUserModal({ deletingUser, onClose }: Props) {
	const { sdk } = useSDK()
	const { t } = useLocaleContext()

	const [hardDelete, setHardDelete] = useState(false)

	const client = useQueryClient()
	const { mutate, isPending } = useGraphQLMutation(mutation, {
		onSuccess: async () => {
			await client.refetchQueries({
				predicate: (query) => query.queryKey[0] === sdk.cacheKeys.users,
			})
			onClose()
		},
	})

	const handleDelete = useCallback(() => {
		if (deletingUser) {
			mutate({ id: deletingUser.id, hardDelete })
		}
	}, [deletingUser, hardDelete, mutate])

	return (
		<Dialog open={!!deletingUser}>
			<Dialog.Content size="sm">
				<Dialog.Header>
					<Dialog.Title>{t(`${LOCALE_NS}.title`)}</Dialog.Title>
					<Dialog.Description>{t(`${LOCALE_NS}.description`)}</Dialog.Description>
					<Dialog.Close onClick={onClose} disabled={isPending} />
				</Dialog.Header>

				<Dialog.Footer className="gap-3 sm:justify-between sm:gap-0 w-full items-center">
					<div className="shrink-0">
						<CheckBox
							label={t(`${LOCALE_NS}.hardDeleteLabel`)}
							checked={hardDelete}
							onClick={() => setHardDelete((prev) => !prev)}
						/>
					</div>

					<div className="space-y-2 sm:flex-row sm:justify-end sm:space-x-2 sm:space-y-0 flex w-full flex-col-reverse space-y-reverse">
						<Button variant="outline" onClick={onClose} disabled={isPending}>
							{t(`${LOCALE_NS}.cancel`)}
						</Button>
						<Button isLoading={isPending} disabled={isPending} onClick={handleDelete}>
							{t(`${LOCALE_NS}.deleteUser`)}
						</Button>
					</div>
				</Dialog.Footer>
			</Dialog.Content>
		</Dialog>
	)
}
