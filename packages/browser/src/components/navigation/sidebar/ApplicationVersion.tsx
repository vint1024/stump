import { useStumpVersion } from '@stump/client'
import { cx, Link, TEXT_VARIANTS } from '@stump/components'
import { useMemo } from 'react'

export default function ApplicationVersion() {
	const version = useStumpVersion()

	// Show the upstream-equivalent base version (v0.1.4); the fork suffix on the
	// build is represented by the exact commit shown alongside it.
	const baseSemver = version?.semver?.split('-')[0]

	const url = useMemo(() => {
		if (!version) return undefined

		const { rev } = version
		const repoUrl = 'https://github.com/vint1024/stump'
		if (baseSemver && baseSemver !== '0.0.0') {
			return `${repoUrl}/releases/tag/v${baseSemver}`
		} else if (rev) {
			return `${repoUrl}/commit/${rev}`
		} else {
			return repoUrl
		}
	}, [version, baseSemver])

	if (!version) return null

	return (
		<Link
			href={url}
			target="__blank"
			rel="noopener noreferrer"
			className={cx('space-x-2 pl-2 flex items-center text-xxs', TEXT_VARIANTS.muted)}
			underline={false}
		>
			<span>
				v{baseSemver}
				{!!version.rev && ` - ${version.rev}`}
			</span>
		</Link>
	)
}
