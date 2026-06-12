import { Dialog, Heading, Popover } from '@stump/components'
import { useLocaleContext } from '@stump/i18n'
import { Paintbrush } from 'lucide-react'

import ControlButton from './ControlButton'
import FontFamily from './FontFamily'
import FontSizeControl from './FontSizeControl'
import LineHeightControl from './LineHeightControl'
import ReadingDirection from './ReadingDirection'
import ReadingMode from './ReadingMode'

export default function ThemeControls() {
	const { t } = useLocaleContext()

	return (
		<Dialog>
			<Dialog.Trigger asChild>
				<ControlButton title={t('components.readers.epub.controls.ThemeControls.title')}>
					<Paintbrush className="h-4 w-4" />
				</ControlButton>
			</Dialog.Trigger>

			<Dialog.Content size="md" className="gap-4 z-101 flex flex-col bg-background-surface">
				<Heading size="md">
					{t('components.readers.epub.controls.ThemeControls.appearance')}
				</Heading>

				<FontFamily />
				<FontSizeControl />
				<LineHeightControl />
				<ReadingDirection />
				<ReadingMode />
			</Dialog.Content>
		</Dialog>
	)

	return (
		<Popover>
			<Popover.Trigger asChild>
				<ControlButton title={t('components.readers.epub.controls.ThemeControls.title')}>
					<Paintbrush className="h-4 w-4" />
				</ControlButton>
			</Popover.Trigger>

			<Popover.Content
				size="sm"
				align="end"
				className="gap-4 z-101 flex flex-col bg-background-surface"
			>
				<FontSizeControl />
				<LineHeightControl />
				<ReadingDirection />
				<ReadingMode />
			</Popover.Content>
		</Popover>
	)
}
