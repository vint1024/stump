import { useGraphQLMutation, useSDK, useSuspenseGraphQL } from '@stump/client'
import { Button, Heading, Text } from '@stump/components'
import { graphql } from '@stump/graphql'
import { useLocaleContext } from '@stump/i18n'
import { useQueryClient } from '@tanstack/react-query'
import { useCallback, useState } from 'react'
import { toast } from 'sonner'

import ContentAccessRulesEditor, { EditableRule } from './ContentAccessRulesEditor'

const query = graphql(`
	query ContentAccessRulesSection($id: ID!) {
		userById(id: $id) {
			id
			contentAccessRules {
				id
				dimension
				mode
				values
				restrictOnUnset
			}
		}
	}
`)

const mutation = graphql(`
	mutation ContentAccessRulesSectionSave($userId: ID!, $rules: [ContentAccessRuleInput!]!) {
		setUserContentAccessRules(userId: $userId, rules: $rules) {
			id
		}
	}
`)

const LOCALE_BASE = 'settingsScene.server/users.createOrUpdateForm.contentRules'

type Props = {
	userId: string
}

/**
 * Admin-managed content access rules for an EXISTING user: loads the saved
 * rules and persists them via its own mutation. The create flow uses
 * {@link ContentAccessRulesEditor} directly, saving as part of user creation.
 */
export default function ContentAccessRulesSection({ userId }: Props) {
	const { sdk } = useSDK()
	const { t } = useLocaleContext()
	const client = useQueryClient()

	const {
		data: { userById },
	} = useSuspenseGraphQL(query, sdk.cacheKey('user', [userId, 'contentRules']), {
		id: userId,
	})

	const [rules, setRules] = useState<EditableRule[]>(
		() =>
			userById?.contentAccessRules.map((rule) => ({
				dimension: rule.dimension,
				mode: rule.mode,
				values: rule.values,
				restrictOnUnset: rule.restrictOnUnset,
			})) ?? [],
	)

	const { mutate: save, isPending } = useGraphQLMutation(mutation, {
		onSuccess: () => {
			toast.success(t(`${LOCALE_BASE}.toasts.saved`))
			client.invalidateQueries({ queryKey: [sdk.cacheKeys.user] })
		},
		onError: (error) => {
			console.error('Failed to save content restrictions', error)
			toast.error(t(`${LOCALE_BASE}.toasts.saveFailed`))
		},
	})

	const handleSave = useCallback(() => {
		save({
			userId,
			rules: rules
				.map((rule) => ({
					dimension: rule.dimension,
					mode: rule.mode,
					restrictOnUnset: rule.restrictOnUnset,
					values: rule.values.map((value) => value.trim()).filter(Boolean),
				}))
				.filter((rule) => rule.values.length > 0),
		})
	}, [save, userId, rules])

	return (
		<div className="gap-y-4 flex flex-col">
			<div>
				<Heading size="sm">{t(`${LOCALE_BASE}.heading`)}</Heading>
				<Text size="sm" variant="muted">
					{t(`${LOCALE_BASE}.description`)}
				</Text>
			</div>

			<ContentAccessRulesEditor rules={rules} onChange={setRules} />

			<div className="flex">
				<Button type="button" disabled={isPending} onClick={handleSave}>
					{t(`${LOCALE_BASE}.save`)}
				</Button>
			</div>
		</div>
	)
}
