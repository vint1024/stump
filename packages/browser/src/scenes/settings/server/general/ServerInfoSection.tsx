import { useStumpVersion } from '@stump/client'
import {
	Alert,
	AlertDescription,
	AlertTitle,
	cn,
	Heading,
	Label,
	Link,
	Text,
	TEXT_VARIANTS,
} from '@stump/components'
import { useLocaleContext } from '@stump/i18n'
import { intlFormat } from 'date-fns'
import { Info } from 'lucide-react'
import { useMemo } from 'react'

const REPO_URL = 'https://github.com/vint1024/stump'
const IS_DEV = import.meta.env.DEV

export default function ServerInfoSection() {
	const version = useStumpVersion()

	const { t } = useLocaleContext()

	// Display the upstream-equivalent semantic version (v0.1.4): the build carries
	// a fork suffix (e.g. 0.1.4-vint-0.3.0); the exact build is the commit below.
	const baseSemver = useMemo(() => version?.semver?.split('-')[0], [version])

	const versionUrl = useMemo(
		() => (baseSemver ? `${REPO_URL}/releases/tag/v${baseSemver}` : REPO_URL),
		[baseSemver],
	)

	const commitUrl = useMemo(
		() => (version?.rev ? `${REPO_URL}/commit/${version.rev}` : undefined),
		[version],
	)

	// The fork ships a single "stable" channel; default to it so the field always
	// shows, and surface it as "NoirPanther (<channel>)" while keeping the raw
	// value for the non-stable warning below.
	const rawChannel = useMemo(
		() => version?.buildChannel || (IS_DEV ? 'local' : 'stable'),
		[version],
	)
	const buildChannel = `NoirPanther (${rawChannel})`

	return (
		<div className="gap-4 flex flex-col">
			<div>
				<Heading size="sm">{t('settingsScene.server/general.sections.serverInfo.title')}</Heading>
				<Text size="sm" variant="muted" className="mt-1">
					{t('settingsScene.server/general.sections.serverInfo.description')}
				</Text>
			</div>

			{rawChannel !== 'stable' && (
				<Alert variant="info">
					<Info className="h-4 w-4" />
					<AlertTitle>
						{t('settingsScene.server/general.sections.serverInfo.nonStableChannel.title')}
					</AlertTitle>
					<AlertDescription className="flex">
						{t('settingsScene.server/general.sections.serverInfo.nonStableChannel.description.0')}{' '}
						<span className="font-semibold">{buildChannel}</span>{' '}
						{t('settingsScene.server/general.sections.serverInfo.nonStableChannel.description.1')}
					</AlertDescription>
				</Alert>
			)}

			<div className="gap-12 md:gap-8 flex flex-row flex-wrap">
				{version && (
					<div>
						<Label>{t('settingsScene.server/general.sections.serverInfo.semanticVersion')}</Label>
						<Link
							href={versionUrl}
							target="__blank"
							rel="noopener noreferrer"
							className={cn(
								'space-x-2 text-sm flex items-center hover:underline',
								TEXT_VARIANTS.muted,
							)}
							underline={false}
						>
							<span>v{baseSemver}</span>
						</Link>
					</div>
				)}

				<div>
					<Label>{t('settingsScene.server/general.sections.serverInfo.buildChannel')}</Label>
					<Text size="sm" variant="muted">
						{buildChannel}
					</Text>
				</div>

				{version && (
					<div>
						<Label>{t('settingsScene.server/general.sections.serverInfo.exactCommit')}</Label>
						<Link
							href={commitUrl}
							target="__blank"
							rel="noopener noreferrer"
							className={cn(
								'space-x-2 text-sm flex items-center hover:underline',
								TEXT_VARIANTS.muted,
							)}
							underline={false}
						>
							<span>{version.rev}</span>
						</Link>
					</div>
				)}

				{version && (
					<div>
						<Label>{t('settingsScene.server/general.sections.serverInfo.buildDate')}</Label>
						<Text size="sm" variant="muted">
							{intlFormat(new Date(version.compileTime), {
								month: 'long',
								day: 'numeric',
								year: 'numeric',
								hour: 'numeric',
								minute: '2-digit',
							})}
						</Text>
					</div>
				)}
			</div>
		</div>
	)
}
