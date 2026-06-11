import { useGraphQLMutation, useSDK, useSuspenseGraphQL } from '@stump/client'
import { Button, Heading, Input, NativeSelect, Text } from '@stump/components'
import { ContentRuleDimension, ContentRuleMode, graphql } from '@stump/graphql'
import { useLocaleContext } from '@stump/i18n'
import { useQueryClient } from '@tanstack/react-query'
import { Plus, X } from 'lucide-react'
import { useCallback, useState } from 'react'
import { toast } from 'sonner'

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

type EditableRule = {
	dimension: ContentRuleDimension
	mode: ContentRuleMode
	/** Comma-separated in the editor, split on save */
	values: string
	restrictOnUnset: boolean
}

const LOCALE_BASE = 'settingsScene.server/users.createOrUpdateForm.contentRules'

type Props = {
	userId: string
}

/**
 * Admin-managed content access rules for a user: per-dimension (tag, publisher,
 * genre) allow- or deny-lists which hide matching content from the user
 * everywhere (browse, search, OPDS)
 */
export default function ContentAccessRulesSection({ userId }: Props) {
	const { sdk } = useSDK()
	const { t } = useLocaleContext()
	const client = useQueryClient()

	const dimensionOptions = [
		{ label: t(`${LOCALE_BASE}.dimensions.tag`), value: ContentRuleDimension.Tag },
		{ label: t(`${LOCALE_BASE}.dimensions.publisher`), value: ContentRuleDimension.Publisher },
		{ label: t(`${LOCALE_BASE}.dimensions.genre`), value: ContentRuleDimension.Genre },
	]
	const modeOptions = [
		{ label: t(`${LOCALE_BASE}.modes.exclude`), value: ContentRuleMode.Exclude },
		{ label: t(`${LOCALE_BASE}.modes.only`), value: ContentRuleMode.Only },
	]

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
				values: rule.values.join(', '),
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

	const patchRule = useCallback((index: number, patch: Partial<EditableRule>) => {
		setRules((current) => current.map((rule, i) => (i === index ? { ...rule, ...patch } : rule)))
	}, [])

	const handleSave = useCallback(() => {
		save({
			userId,
			rules: rules
				.map((rule) => ({
					dimension: rule.dimension,
					mode: rule.mode,
					restrictOnUnset: rule.restrictOnUnset,
					values: rule.values
						.split(',')
						.map((value) => value.trim())
						.filter(Boolean),
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

			{rules.map((rule, index) => (
				<div key={index} className="gap-2 flex flex-wrap items-center">
					<NativeSelect
						className="w-36"
						value={rule.dimension}
						options={dimensionOptions}
						onChange={(e) =>
							patchRule(index, { dimension: e.target.value as ContentRuleDimension })
						}
					/>
					<NativeSelect
						className="w-40"
						value={rule.mode}
						options={modeOptions}
						onChange={(e) => patchRule(index, { mode: e.target.value as ContentRuleMode })}
					/>
					<Input
						variant="primary"
						containerClassName="w-72 max-w-full"
						placeholder={t(`${LOCALE_BASE}.valuesPlaceholder`)}
						value={rule.values}
						onChange={(e) => patchRule(index, { values: e.target.value })}
					/>
					{rule.mode === ContentRuleMode.Only && (
						<label className="gap-1.5 text-sm flex cursor-pointer items-center text-foreground-muted">
							<input
								type="checkbox"
								checked={rule.restrictOnUnset}
								onChange={(e) => patchRule(index, { restrictOnUnset: e.target.checked })}
							/>
							{t(`${LOCALE_BASE}.restrictOnUnset`)}
						</label>
					)}
					<Button
						size="icon"
						type="button"
						title={t(`${LOCALE_BASE}.removeRule`)}
						onClick={() => setRules((current) => current.filter((_, i) => i !== index))}
					>
						<X className="h-4 w-4 text-foreground-muted" />
					</Button>
				</div>
			))}

			<div className="gap-2 flex items-center">
				<Button
					type="button"
					variant="outline"
					size="sm"
					onClick={() =>
						setRules((current) => [
							...current,
							{
								dimension: ContentRuleDimension.Tag,
								mode: ContentRuleMode.Exclude,
								values: '',
								restrictOnUnset: false,
							},
						])
					}
				>
					<Plus className="mr-2 h-4 w-4" />
					{t(`${LOCALE_BASE}.addRule`)}
				</Button>
				<Button type="button" disabled={isPending} onClick={handleSave}>
					{t(`${LOCALE_BASE}.save`)}
				</Button>
			</div>
		</div>
	)
}
