import { createContext, useContext } from 'react'

import { AllowedLocale } from './config'

export type LocaleContextProps = {
	locale: AllowedLocale
	t: (key: string, options?: Record<string, unknown>) => string
}

export const getDefaultLocale = (defaultValue: AllowedLocale = 'en-US') => {
	// navigator.language can be undefined on React Native, so fall back to the
	// default rather than returning undefined (which downstream locale handling
	// would crash on)
	const fromNavigator =
		'navigator' in globalThis ? (navigator?.language as AllowedLocale | undefined) : undefined
	return fromNavigator || defaultValue
}

export const LocaleContext = createContext<LocaleContextProps>({
	locale: getDefaultLocale(),
	t: (key: string) => key,
})
export const useLocaleContext = () => useContext(LocaleContext)
