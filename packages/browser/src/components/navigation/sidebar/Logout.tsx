import { ConfirmationModal, IconButton, ToolTip, useBoolean } from '@stump/components'
import { useLocaleContext } from '@stump/i18n'
import { LogOut } from 'lucide-react'

import { useAppContext } from '@/context'

type Props = {
	trigger?: (setOpen: (state: boolean) => void) => React.ReactElement
}

export default function Logout({ trigger }: Props) {
	const { t } = useLocaleContext()
	const { logout } = useAppContext()
	const [isOpen, { on, off }] = useBoolean()

	async function handleLogout() {
		off()
		logout()
	}

	return (
		<ConfirmationModal
			title={t('components.navigation.sidebar.Logout.title')}
			description={t('components.navigation.sidebar.Logout.description')}
			confirmText={t('components.navigation.sidebar.Logout.confirmText')}
			confirmVariant="danger"
			isOpen={isOpen}
			onClose={off}
			onConfirm={handleLogout}
			trigger={
				<ToolTip content={t('components.navigation.sidebar.Logout.tooltip')}>
					{trigger ? (
						trigger(on)
					) : (
						<IconButton
							className="hover:text-foreground-500 text-foreground-subtle"
							onClick={on}
							aria-label={t('components.navigation.sidebar.Logout.tooltip')}
						>
							<LogOut />
						</IconButton>
					)}
				</ToolTip>
			}
		/>
	)
}
