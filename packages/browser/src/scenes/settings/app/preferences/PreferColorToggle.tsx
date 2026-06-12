import { WideSwitch } from '@stump/components'
import { useLocaleContext } from '@stump/i18n'

// TODO: Implement this
export default function PreferColorToggle() {
	const { t } = useLocaleContext()
	const handleChange = () => {}

	return (
		<WideSwitch
			label={t('scenes.settings.app.preferences.PreferColorToggle.label')}
			description={t('scenes.settings.app.preferences.PreferColorToggle.description')}
			checked
			onCheckedChange={handleChange}
			disabled
			formId="prefer_accent_color"
			title={t('scenes.settings.app.preferences.PreferColorToggle.title')}
		/>
	)
}
