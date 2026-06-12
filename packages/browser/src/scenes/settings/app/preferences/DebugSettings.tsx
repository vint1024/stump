import { Label, RawSwitch } from '@stump/components'
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
		<div className="gap-4 flex flex-col">
			<div>
				<h3 className="text-base font-medium text-foreground">
					{t('scenes.settings.app.preferences.DebugSettings.heading')}
				</h3>
				<p className="text-sm text-foreground-muted">
					{t('scenes.settings.app.preferences.DebugSettings.description')}
				</p>
			</div>

			<div className="gap-2 flex flex-col">
				<Label className="rounded-lg p-3 flex items-center justify-between border border-dashed border-fill-brand/40 bg-fill-brand-secondary">
					<div className="gap-1 flex flex-col">
						<span>{t('scenes.settings.app.preferences.DebugSettings.queryToolsLabel')}</span>
						<p className="text-sm text-foreground-muted">
							{t('scenes.settings.app.preferences.DebugSettings.queryToolsDescription')}
						</p>
					</div>
					<RawSwitch
						id="showQueryTools"
						className="data-[state=checked]:bg-fill-brand-secondary/60 data-[state=unchecked]:bg-fill-brand-secondary"
						checked={store.showQueryTools}
						onCheckedChange={(checked) => store.patch({ showQueryTools: checked })}
					/>
				</Label>
			</div>
		</div>
	)
}
