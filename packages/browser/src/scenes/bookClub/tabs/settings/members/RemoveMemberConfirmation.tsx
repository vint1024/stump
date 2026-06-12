import { ConfirmationModal } from '@stump/components'
import { useLocaleContext } from '@stump/i18n'

type Props = {
	isOpen: boolean
	onClose: (didConfirm: boolean) => void
}

export default function RemoveMemberConfirmation({ isOpen, onClose }: Props) {
	const { t } = useLocaleContext()

	return (
		<ConfirmationModal
			isOpen={isOpen}
			onConfirm={() => onClose(true)}
			onClose={() => onClose(false)}
			title={t('scenes.bookClub.tabs.settings.members.RemoveMemberConfirmation.title')}
			description={t('scenes.bookClub.tabs.settings.members.RemoveMemberConfirmation.description')}
			confirmText={t('scenes.bookClub.tabs.settings.members.RemoveMemberConfirmation.confirm')}
			confirmVariant="danger"
			size="sm"
		/>
	)
}
