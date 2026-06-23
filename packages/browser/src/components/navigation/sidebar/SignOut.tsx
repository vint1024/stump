import { Button, ConfirmationModal, Text, useBoolean } from '@stump/components'
import { useLocaleContext } from '@stump/i18n'
import { LogOut } from 'lucide-react'

import { useAppContext } from '@/context'

export default function SignOut() {
	const { t } = useLocaleContext()
	const { logout } = useAppContext()
	const [isOpen, { on, off }] = useBoolean()

	async function handleLogout() {
		off()
		logout()
	}

	return (
		<ConfirmationModal
			title={t('components.navigation.sidebar.SignOut.title')}
			description={t('components.navigation.sidebar.SignOut.description')}
			confirmText={t('components.navigation.sidebar.SignOut.confirmText')}
			confirmVariant="destructive"
			isOpen={isOpen}
			onClose={off}
			onConfirm={handleLogout}
			trigger={
				<Button
					variant="ghost"
					className="h-9 gap-1.5 px-3 w-full justify-start text-destructive hover:bg-destructive/10 hover:text-destructive"
					onClick={on}
				>
					<LogOut className="h-4 w-4" />
					<Text size="sm" className="text-inherit select-none">
						{t('components.navigation.sidebar.SignOut.label')}
					</Text>
				</Button>
			}
		/>
	)
}
