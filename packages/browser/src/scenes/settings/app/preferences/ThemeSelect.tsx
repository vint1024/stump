import { NativeSelect, NewCard } from '@stump/components'
import { useLocaleContext } from '@stump/i18n'

import { useTheme } from '@/hooks'

const localeKey = 'settingsScene.app/preferences.sections.themeSelect'

// TODO: We officially have enough themes to warrant a filterable combobox IMO, so do that
export default function ThemeSelect() {
	const { t } = useLocaleContext()
	const { theme, changeTheme } = useTheme()

	return (
		<NewCard.Row label={t(`${localeKey}.label`)} className="flex-row items-center justify-between">
			<div className="w-52 max-w-full">
				<NativeSelect
					size="sm"
					className="enabled:hover:bg-muted/40"
					value={theme}
					options={[
						{ label: t(`${localeKey}.options.system`), value: 'system' },
						// NoirPanther brand themes (default is "vibranium")
						{ label: t(`${localeKey}.options.vibranium`), value: 'vibranium' },
						{ label: t(`${localeKey}.options.gold`), value: 'gold' },
						{ label: t(`${localeKey}.options.emerald`), value: 'emerald' },
						{ label: t(`${localeKey}.options.noir`), value: 'noir' },
						{ label: t(`${localeKey}.options.vibranium-light`), value: 'vibranium-light' },
						{ label: t(`${localeKey}.options.gold-light`), value: 'gold-light' },
						// The original Stump look that follows the OS light/dark setting.
						{ label: t(`${localeKey}.options.vanilla`), value: 'vanilla' },
						{ label: t(`${localeKey}.options.light`), value: 'light' },
						{ label: t(`${localeKey}.options.dark`), value: 'dark' },
						{ label: t(`${localeKey}.options.bronze`), value: 'bronze' },
						{ label: t(`${localeKey}.options.ocean`), value: 'ocean' },
						{ label: t(`${localeKey}.options.autumn`), value: 'autumn' },
						{ label: t(`${localeKey}.options.cosmic`), value: 'cosmic' },
						{ label: t(`${localeKey}.options.pumpkin`), value: 'pumpkin' },
						{ label: t(`${localeKey}.options.midnight`), value: 'midnight' },
					]}
					onChange={(e) => changeTheme(e.target.value)}
				/>
			</div>
		</NewCard.Row>
	)
}
