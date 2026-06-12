import { ConfirmationModal, Text, useBoolean } from '@stump/components'
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
			confirmVariant="danger"
			isOpen={isOpen}
			onClose={off}
			onConfirm={handleLogout}
			trigger={
				<button
					className="gap-1.5 px-2 flex h-[2.35rem] w-full items-center bg-sidebar-overlay/50 text-foreground-subtle transition-colors duration-150 outline-none hover:bg-sidebar-overlay-hover"
					onClick={on}
				>
					<LogOut className="h-4 w-4" />
					<Text size="sm" className="select-none">
						{t('components.navigation.sidebar.SignOut.label')}
					</Text>
				</button>
			}
		/>
	)
}
