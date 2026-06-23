import { Heading, Text } from '@stump/components'
import { useLocaleContext } from '@stump/i18n'
import { Suspense } from 'react'

import ClearLoginActivityConfirmation from './ClearActivityConfirmation'
import LoginActivityTable from './LoginActivityTable'

export default function LoginActivitySection() {
	const { t } = useLocaleContext()

	return (
		<div className="gap-y-4 flex flex-col">
			<div className="flex items-end justify-between">
				<div>
					<Heading size="sm">
						{t('scenes.settings.server.users.login-activity.LoginActivitySection.heading')}
					</Heading>
					<Text size="sm" variant="muted" className="mt-1">
						{t('scenes.settings.server.users.login-activity.LoginActivitySection.description')}
					</Text>
				</div>

				<ClearLoginActivityConfirmation />
			</div>

			<div className="gap-3 flex flex-col">
				<Suspense>
					<LoginActivityTable />
				</Suspense>
			</div>
		</div>
	)
}
