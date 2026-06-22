import { zodResolver } from '@hookform/resolvers/zod'
import { queryClient, useLoginOrRegister, useOidcConfig, useSDK } from '@stump/client'
import {
	Alert,
	AlertDescription,
	Button,
	cx,
	Form,
	Heading,
	Input,
	PasswordInput,
} from '@stump/components'
import { useLocaleContext } from '@stump/i18n'
import { isAxiosError } from '@stump/sdk'
import { motion, Variants } from 'framer-motion'
import { ArrowRight, ShieldAlert } from 'lucide-react'
import { useCallback, useRef, useState } from 'react'
import { FieldValues, useForm } from 'react-hook-form'
import { useNavigate } from 'react-router'
import { useSearchParams } from 'react-router-dom'
import { toast } from 'sonner'
import { z } from 'zod'

import { useAppStore, useUserStore } from '@/stores'

// TODO: redirect away if the user is already logged in
export default function LoginOrClaimScene() {
	const navigate = useNavigate()

	const [params] = useSearchParams()
	const [redirect] = useState(() => params.get('redirect') || '/')

	const [showServers, setShowServers] = useState(false)

	const setUser = useUserStore((store) => store.setUser)
	const isDesktop = useAppStore((store) => store.platform !== 'browser')

	const { sdk } = useSDK()
	const { t } = useLocaleContext()
	const {
		isClaimed,
		isCheckingClaimed,
		loginUser,
		registerUser,
		isLoggingIn,
		isRegistering,
		loginError,
	} = useLoginOrRegister({
		onSuccess: async (user) => {
			setUser(user)
			await queryClient.refetchQueries({
				queryKey: [sdk.auth.keys.me],
				exact: false,
			})
			if (redirect.includes('/swagger') || redirect.includes('/api')) {
				// eslint-disable-next-line react-compiler/react-compiler
				window.location.href = redirect
			} else {
				navigate(redirect, { replace: true })
			}
		},
		refetchClaimed: !showServers,
	})
	const oidcConfig = useOidcConfig()

	// The wordmark is a neon sign: lit (its steady glow) for a random spell of
	// up to ~15s, then fully off — no glow, dimmed — for a random 1–3s, then lit
	// again, like a faulty tube. A callback ref runs the cycle the moment the
	// wordmark mounts (it only renders once the server is claimed) and stops it
	// on unmount. Skipped entirely under prefers-reduced-motion (stays lit).
	// The ambient wash on the "wall" behind the sign — its light, so it fades out
	// with the tube and back in when the sign relights.
	const wallGlowRef = useRef<HTMLDivElement>(null)
	const stopSignRef = useRef<(() => void) | null>(null)
	const wordmarkRef = useCallback((el: HTMLHeadingElement | null) => {
		stopSignRef.current?.()
		stopSignRef.current = null
		if (!el) return
		if (window.matchMedia?.('(prefers-reduced-motion: reduce)').matches) return

		let active = true
		let timer: ReturnType<typeof setTimeout>

		const lightUp = () => {
			if (!active) return
			el.classList.remove('neon-off')
			if (wallGlowRef.current) wallGlowRef.current.style.opacity = '1'
			timer = setTimeout(turnOff, 4000 + Math.random() * 11000) // lit 4–15s
		}
		const turnOff = () => {
			if (!active) return
			el.classList.add('neon-off')
			if (wallGlowRef.current) wallGlowRef.current.style.opacity = '0'
			timer = setTimeout(lightUp, 1000 + Math.random() * 2000) // off 1–3s
		}

		lightUp() // start lit

		stopSignRef.current = () => {
			active = false
			clearTimeout(timer)
			el.classList.remove('neon-off')
			if (wallGlowRef.current) wallGlowRef.current.style.opacity = '1'
		}
	}, [])

	const schema = z.object({
		password: z.string().min(1, { message: t('authScene.form.validation.missingPassword') }),
		username: z.string().min(1, { message: t('authScene.form.validation.missingUsername') }),
	})

	const form = useForm<z.infer<typeof schema>>({
		resolver: zodResolver(schema),
	})

	const login = useCallback(
		async ({ username, password }: FieldValues) => {
			try {
				await loginUser({ password, username })
			} catch (error) {
				console.error('Error logging in:', error)
				toast.error(t('authScene.toasts.loginFailed'))
			}
		},
		[loginUser, t],
	)

	const handleSubmit = useCallback(
		async ({ username, password }: FieldValues) => {
			if (isClaimed) {
				await login({ password, username })
			} else {
				try {
					await registerUser({ password, username })
					await login({ password, username })
				} catch (error) {
					console.error('Error registering', error)
					toast.error(t('authScene.toasts.registrationFailed'))
				}
			}
		},
		[isClaimed, login, registerUser, t],
	)

	const handleOidcLogin = useCallback(() => {
		const authorizeUrl = sdk.auth.getOidcAuthorizeUrl()
		window.location.href = authorizeUrl
	}, [sdk.auth])

	const renderHeader = () => {
		if (isClaimed) {
			return (
				<div className="gap-4 px-2 flex shrink-0 items-center justify-center">
					<img src="/assets/favicon.png" width="80" height="80" />
					<Heading
						ref={wordmarkRef}
						variant="gradient"
						size="3xl"
						// Neon sign in the theme accent (vibranium by default): the static
						// drop-shadow is the steady glow; the effect above toggles `.neon-off`
						// to switch the tube off for a beat now and then.
						className="neon-sign font-bold [filter:drop-shadow(0_0_6px_var(--primary))_drop-shadow(0_0_18px_var(--primary))]"
					>
						NoirPanther
					</Heading>
				</div>
			)
		} else {
			return (
				<div className="sm:max-w-md md:max-w-lg text-left">
					<h1 className="text-4xl font-semibold text-foreground">{t('authScene.claimHeading')}</h1>
					<p className="mt-1.5 text-base text-foreground">{t('authScene.claimText')}</p>
				</div>
			)
		}
	}

	const renderError = () => {
		if (!loginError) return null

		// If the response is a 403, and we are NOT claiming, it is likely because
		// the account login is disabled (i.e. the account is locked). Additionally,
		// authentication had to have passed, otherwise we would have gotten a 401. So,
		// we can safely display the error message from the server.
		if (isAxiosError(loginError) && loginError.response?.status === 403) {
			const message = loginError.response?.data as string
			return (
				<Alert variant="destructive" className="sm:max-w-md md:max-w-lg">
					<ShieldAlert />
					<AlertDescription>{message || 'An unknown error occurred'}</AlertDescription>
				</Alert>
			)
		}

		return null
	}

	if (isCheckingClaimed) {
		return null
	}

	return (
		<div data-tauri-drag-region className="flex h-screen w-screen items-center bg-background">
			<motion.div
				// @ts-expect-error: It's fine
				className="w-screen shrink-0"
				animate={showServers ? 'appearOut' : 'appearIn'}
				variants={variants}
			>
				<div className="gap-8 p-4 relative flex h-full w-full flex-col items-center justify-center overflow-hidden bg-background">
					{/* Ambient neon wash — the lit wordmark spilling onto the "wall"
					    behind it, centred a touch above the middle where the sign sits.
					    Its opacity is toggled with the sign (see wordmarkRef) so the wall
					    goes dark when the tube switches off. */}
					<div
						ref={wallGlowRef}
						aria-hidden
						className="inset-0 pointer-events-none absolute"
						style={{
							background:
								'radial-gradient(46% 42% at 50% 39%, rgba(139,92,246,0.20), rgba(139,92,246,0.06) 48%, transparent 72%)',
							transition: 'opacity 240ms ease',
						}}
					/>
					{renderHeader()}
					{renderError()}

					<Form
						form={form}
						onSubmit={handleSubmit}
						className={cx(
							{ 'sm:max-w-md md:max-w-lg w-full': !isClaimed },
							{ 'min-w-[20rem]': isClaimed },
						)}
					>
						{!oidcConfig.disableLocalAuth && (
							<>
								<Input
									id="username"
									label={t('authScene.form.labels.username')}
									autoComplete="username"
									autoCapitalize="off"
									autoFocus
									fullWidth
									{...form.register('username')}
								/>

								<PasswordInput
									id="password"
									label={t('authScene.form.labels.password')}
									type="password"
									autoComplete="current-password"
									fullWidth
									{...form.register('password')}
								/>

								<Button type="submit" isLoading={isLoggingIn || isRegistering} className="mt-2">
									{isClaimed
										? t('authScene.form.buttons.login')
										: t('authScene.form.buttons.createAccount')}
								</Button>
							</>
						)}

						{oidcConfig.enabled && (
							<>
								{!oidcConfig.disableLocalAuth && (
									<div className="my-1.5 relative">
										<div className="inset-0 absolute flex items-center">
											<div className="w-full border-t border-border" />
										</div>
										<div className="text-xs relative flex justify-center uppercase">
											<span className="px-2 bg-background text-muted-foreground">Or</span>
										</div>
									</div>
								)}

								<Button
									type="button"
									variant="outline"
									onClick={handleOidcLogin}
									className="w-full"
								>
									{isClaimed
										? t('authScene.form.buttons.loginWithOidc')
										: t('authScene.form.buttons.createAccountWithOidc')}
								</Button>
							</>
						)}

						{isDesktop && (
							<button
								className="group p-4 hover:border-opacity-70 flex w-full items-center justify-between border-l border-border transition-colors duration-100 hover:border-border hover:bg-muted/50"
								type="button"
								onClick={() => setShowServers(true)}
							>
								<span className="text-sm font-semibold text-muted-foreground transition-colors duration-100 group-hover:text-foreground">
									{t('common.goToServers')}
								</span>

								<ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-foreground" />
							</button>
						)}
					</Form>
				</div>
			</motion.div>
		</div>
	)
}

const variants: Variants = {
	appearIn: {
		display: 'block',
		opacity: 1,
		scale: 1,
		transition: {
			damping: 20,
			delayChildren: 0.3,
			stiffness: 150,
			type: 'spring',
		},
	},
	appearOut: {
		display: 'none',
		opacity: 0,
		scale: 0.8,
	},
}
