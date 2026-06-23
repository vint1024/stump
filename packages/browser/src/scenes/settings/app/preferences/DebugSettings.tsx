import { NewCard, RawSwitch } from '@stump/components'
import { useLocaleContext } from '@stump/i18n'

import { useDebugStore } from '@/stores'

const IS_DEVELOPMENT = import.meta.env.DEV

export default function Container() {
	if (!IS_DEVELOPMENT) return null

	return <DebugSettings />
}

function DebugSettings() {
	const { t } = useLocaleContext()
	const store = useDebugStore()

	return (
		<NewCard
			tone="debug"
			label={t('scenes.settings.app.preferences.DebugSettings.heading')}
			description={t('scenes.settings.app.preferences.DebugSettings.description')}
		>
			<NewCard.Row
				label={t('scenes.settings.app.preferences.DebugSettings.queryToolsLabel')}
				description={t('scenes.settings.app.preferences.DebugSettings.queryToolsDescription')}
				onClick={() => store.patch({ showQueryTools: !store.showQueryTools })}
			>
				<RawSwitch
					id="showQueryTools"
					className="data-[state=checked]:bg-debug/70 data-[state=unchecked]:bg-debug/30"
					checked={store.showQueryTools}
					onCheckedChange={(checked) => store.patch({ showQueryTools: checked })}
				/>
			</NewCard.Row>
		</NewCard>
	)
}
