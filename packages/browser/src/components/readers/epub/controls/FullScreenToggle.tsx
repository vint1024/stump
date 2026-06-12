import { useLocaleContext } from '@stump/i18n'
import { Fullscreen } from 'lucide-react'

import { useEpubReaderControls } from '../context'
import ControlButton from './ControlButton'

export default function FullScreenToggle() {
	const { t } = useLocaleContext()
	const { fullscreen, setFullscreen } = useEpubReaderControls()

	return (
		<ControlButton
			title={
				fullscreen
					? t('components.readers.epub.controls.FullScreenToggle.exitFullscreen')
					: t('components.readers.epub.controls.FullScreenToggle.enterFullscreen')
			}
		>
			<Fullscreen className="h-4 w-4" onClick={() => setFullscreen(!fullscreen)} />
		</ControlButton>
	)
}
