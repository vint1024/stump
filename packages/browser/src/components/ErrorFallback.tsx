import { Button, ButtonOrLink, useBodyLock } from '@stump/components'
import { useLocaleContext } from '@stump/i18n'
import { ExternalLink } from 'lucide-react'
import { FallbackProps } from 'react-error-boundary'
import { toast } from 'sonner'

import { copyTextToClipboard } from '../utils/misc'

// TODO: take in platform?
export function ErrorFallback({ error, resetErrorBoundary }: FallbackProps) {
	const { t } = useLocaleContext()
	useBodyLock()

	function copyErrorStack() {
		if (error.stack) {
			copyTextToClipboard(error.stack).then(() => {
				toast.success(t('components.ErrorFallback.copiedToClipboard'))
			})
		}
	}

	return (
		<div
			data-tauri-drag-region
			className="flex h-full w-full flex-col items-center justify-center overflow-hidden"
		>
			<img
				src="/assets/svg/bomb.svg"
				alt={t('components.ErrorFallback.imageAlt')}
				className="max-h-64 sm:inline-block mx-auto hidden w-1/2 shrink-0 object-scale-down"
			/>
			<div className="max-w-sm sm:max-w-md md:max-w-xl">
				<div className="text-left">
					<h1 className="text-4xl font-semibold text-foreground">
						{t('components.ErrorFallback.criticalError')}
					</h1>
					<p className="mt-1.5 text-lg text-foreground">
						{error.message || t('components.ErrorFallback.emptyMessage')}
					</p>
				</div>
				<div className="gap-3 pt-3 flex w-full items-center">
					<ButtonOrLink
						onClick={resetErrorBoundary}
						title={t('components.ErrorFallback.goHomeTitle')}
						forceAnchor
						href="/"
					>
						{t('components.ErrorFallback.goHome')}
					</ButtonOrLink>
					<ButtonOrLink
						title={t('components.ErrorFallback.reportBugTitle')}
						href="https://github.com/stumpapp/stump/issues/new/choose"
						target="_blank"
					>
						{t('components.ErrorFallback.reportBug')} <ExternalLink className="ml-2 h-4 w-4" />
					</ButtonOrLink>
					{error.stack && (
						<Button
							title={t('components.ErrorFallback.copyDetailsTitle')}
							onClick={copyErrorStack}
							variant="ghost"
						>
							{t('components.ErrorFallback.copyDetails')}
						</Button>
					)}
				</div>
			</div>
		</div>
	)
}
