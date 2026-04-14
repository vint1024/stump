import { queryClient } from '@stump/client'
import { Api, checkOPDSURL, checkUrl, formatApiURL } from '@stump/sdk'
import { useQuery } from '@tanstack/react-query'
import { useRouter } from 'expo-router'
import { KeyRound, Shield, ShieldX, Sliders, SquareX, Trash } from 'lucide-react-native'
import { View } from 'react-native'
import { match } from 'ts-pattern'

import { useColors } from '~/lib/constants'
import { useTranslate } from '~/lib/hooks'
import { cn } from '~/lib/utils'
import { usePreferencesStore } from '~/stores'
import { useCacheStore } from '~/stores/cache'
import { SavedServer, useSavedServers } from '~/stores/savedServer'

import { Icon, Text } from '../ui'
import { ContextMenu } from '../ui/context-menu/context-menu'

type Props = {
	server: SavedServer
	onEdit: () => void
	onDelete: () => void
	forceOPDS?: boolean
}

const checkServerUrl = (server: SavedServer) => {
	if (server.kind === 'stump') {
		return checkUrl(formatApiURL(server.url, 'v2'))
	}

	return checkOPDSURL(server.url)
}

export default function SavedServerListItem({ server, onEdit, onDelete, forceOPDS }: Props) {
	const { t } = useTranslate()

	const { data: successfulStatus, isLoading } = useQuery({
		queryFn: async () => await checkServerUrl(server),
		queryKey: ['ping', server.url, server.name],
	})

	// first load does not count
	const isReachable = !isLoading && successfulStatus

	const maskURLs = usePreferencesStore((state) => state.maskURLs)

	const formatURL = (url: string) => {
		try {
			const urlObj = new URL(url)
			const domain = urlObj.hostname

			return maskURLs ? domain.replace(/./g, '*') : domain
			// return maskURLs
			// 	? `${urlObj.protocol}//${host.replace(domain, domain.replace(/./g, '*'))}`
			// 	: `${urlObj.protocol}//${host}`
		} catch {
			return maskURLs ? url.replace(/./g, '*') : url
		}
	}

	const colors = useColors()

	const renderUrlSection = () => {
		// lock or globe icon depending on if the url is https or not
		const isSecure = server.url.startsWith('https://')
		const icon = isSecure ? Shield : ShieldX

		return (
			<View className="gap-2 flex-row items-center">
				<View>
					<View
						className={cn(
							'p-0.5 squircle rounded-xl relative grow dark:bg-transparent',
							isSecure ? 'bg-fill-success-secondary' : 'bg-fill-warning-secondary',
						)}
					>
						<View className="squircle h-6 w-6 flex shrink-0 items-center justify-center">
							<Icon
								as={icon}
								size={14}
								strokeWidth={1.8}
								absoluteStrokeWidth
								color={isSecure ? colors.fill.success.DEFAULT : colors.fill.warning.DEFAULT}
							/>
						</View>
					</View>
				</View>

				<Text size="default" className="text-foreground-muted">
					{formatURL(server.url)}
				</Text>
			</View>
		)
	}

	const { deleteServerToken } = useSavedServers()

	const deleteCachedSdk = useCacheStore((state) => state.removeSDK)
	const cachedServerSdk = useCacheStore((state) => state.sdks[server.id] as Api | undefined)

	const onClearCache = () => {
		// We can assume no SDK means no cache
		if (cachedServerSdk) {
			queryClient.removeQueries({
				exact: false,
				predicate: ({ queryKey }) => queryKey.includes(server.id),
			})
		}
	}

	const router = useRouter()

	const serverPath = match(server.kind)
		.with('stump', () => (forceOPDS ? '/opds/[id]' : '/server/[id]'))
		.with('opds', () => '/opds/[id]')
		.with('opds-legacy', () => '/opds-legacy/[id]')
		.exhaustive()

	return (
		<View className="w-full">
			<ContextMenu
				onPress={() =>
					router.push({
						// @ts-expect-error: It's fine
						pathname: serverPath,
						params: {
							id: server.id,
						},
					})
				}
				groups={[
					{
						items: [
							{
								label: t('common.edit'),
								icon: {
									ios: 'slider.horizontal.2.square.on.square',
									android: Sliders,
								},
								onPress: onEdit,
							},
							{
								label: t('savedServerActions.clearCache'),
								icon: {
									ios: 'clear',
									android: SquareX,
								},
								onPress: onClearCache,
								disabled: !cachedServerSdk,
							},
							...(server.kind === 'stump' && !forceOPDS
								? [
										{
											label: t('savedServerActions.discardTokens.label'),
											subtext: t('savedServerActions.discardTokens.description'),
											icon: {
												ios: 'key.fill',
												android: KeyRound,
											},
											onPress: async () => {
												await deleteServerToken(server.id)
												const idsToDelete = [
													server.id,
													...(server.stumpOPDS ? [`${server.id}-opds`] : []),
												]
												idsToDelete.forEach((id) => deleteCachedSdk(id))
											},
										} as const,
									]
								: []),
							{
								label: t('common.delete'),
								icon: {
									ios: 'trash',
									android: Trash,
								},
								onPress: onDelete,
								role: 'destructive',
							},
						],
					},
				]}
			>
				<View className="squircle ios:rounded-[2rem] rounded-3xl bg-black/5 dark:bg-white/10 p-4 w-full items-start border border-edge">
					<View className="gap-2 flex-1 items-start justify-center">
						<View className="gap-3 relative flex w-full flex-row items-center justify-between">
							<Text size="lg" className="tracking-wide">
								{server.name}
							</Text>

							{!isLoading && (
								<View className="h-4 w-4 relative items-center justify-center">
									<View
										className={cn(
											'h-4 w-4 items-center justify-center rounded-full',
											isReachable ? 'bg-fill-success-secondary' : 'bg-fill-danger-secondary',
										)}
									/>

									<View
										className={cn(
											'h-2 w-2 absolute rounded-full',
											isReachable ? 'bg-fill-success-secondary' : 'bg-fill-danger-secondary',
										)}
									/>
								</View>
							)}
						</View>

						{renderUrlSection()}
					</View>
				</View>
			</ContextMenu>
		</View>
	)
}
