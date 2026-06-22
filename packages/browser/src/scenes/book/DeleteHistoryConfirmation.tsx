import { Alert, AlertDescription, AlertTitle, ConfirmationModal } from '@stump/components'
import { useLocaleContext } from '@stump/i18n'
import { AlertTriangle } from 'lucide-react'

type Props = {
	isOpen: boolean
	onCancel: () => void
	onConfirm: () => void
}

export default function DeleteHistoryConfirmation({ isOpen, onCancel, onConfirm }: Props) {
	const { t } = useLocaleContext()

	return (
		<ConfirmationModal
			title={t('scenes.book.DeleteHistoryConfirmation.title')}
			description={t('scenes.book.DeleteHistoryConfirmation.description')}
			isOpen={isOpen}
			onClose={onCancel}
			onConfirm={onConfirm}
			confirmVariant="destructive"
		>
			<Alert>
				<AlertTriangle />
				<AlertTitle>{t('scenes.book.DeleteHistoryConfirmation.alertTitle')}</AlertTitle>
				<AlertDescription>
					{t('scenes.book.DeleteHistoryConfirmation.alertDescription')}
				</AlertDescription>
			</Alert>
		</ConfirmationModal>
	)
}
