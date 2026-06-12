import { useGraphQLMutation } from '@stump/client'
import { ConfirmationModal } from '@stump/components'
import { graphql } from '@stump/graphql'
import { useLocaleContext } from '@stump/i18n'
import { useNavigate } from 'react-router'
import { toast } from 'sonner'

import paths from '../../paths'

const mutation = graphql(`
	mutation DeleteBookClubConfirmation($id: ID!) {
		deleteBookClub(id: $id) {
			id
		}
	}
`)

type Props = {
	id: string
	onClose: () => void
	isOpen: boolean
	trigger?: React.ReactNode
}

export default function DeleteBookClubConfirmation({ isOpen, id, onClose, trigger }: Props) {
	const { t } = useLocaleContext()
	const navigate = useNavigate()

	const { mutate: deleteClub, isPending } = useGraphQLMutation(mutation, {
		onSuccess: () => {
			onClose()
			navigate(paths.bookClubs())
		},
		onError: (error) => {
			console.error('Error deleting book club:', error)
			toast.error(t('components.bookClub.DeleteBookClubConfirmation.deleteError'))
		},
	})

	return (
		<ConfirmationModal
			title={t('components.bookClub.DeleteBookClubConfirmation.title')}
			description={t('components.bookClub.DeleteBookClubConfirmation.description')}
			confirmText={t('components.bookClub.DeleteBookClubConfirmation.confirmText')}
			confirmVariant="danger"
			isOpen={isOpen}
			onClose={onClose}
			onConfirm={() => deleteClub({ id })}
			confirmIsLoading={isPending}
			trigger={trigger}
		/>
	)
}
