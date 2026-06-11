import { useGraphQLMutation, useSDK, useSuspenseGraphQL } from '@stump/client'
import { Button, Heading, Input, NativeSelect, Text } from '@stump/components'
import { ContentRuleDimension, ContentRuleMode, graphql } from '@stump/graphql'
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

const DIMENSION_OPTIONS = [
	{ label: 'Tag', value: ContentRuleDimension.Tag },
	{ label: 'Publisher', value: ContentRuleDimension.Publisher },
	{ label: 'Genre', value: ContentRuleDimension.Genre },
]

const MODE_OPTIONS = [
	{ label: 'Exclude listed', value: ContentRuleMode.Exclude },
	{ label: 'Only listed', value: ContentRuleMode.Only },
]

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
				values: rule.values.join(', '),
				restrictOnUnset: rule.restrictOnUnset,
			})) ?? [],
	)

	const { mutate: save, isPending } = useGraphQLMutation(mutation, {
		onSuccess: () => {
			toast.success('Content restrictions saved')
			client.invalidateQueries({ queryKey: [sdk.cacheKeys.user] })
		},
		onError: (error) => {
			console.error('Failed to save content restrictions', error)
			toast.error('Failed to save content restrictions')
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
				<Heading size="sm">Content restrictions</Heading>
				<Text size="sm" variant="muted">
					Hide content from this user by tag, publisher or genre. Rules combine — an item must pass
					every rule. Tags are inherited from the series and library
				</Text>
			</div>

			{rules.map((rule, index) => (
				<div key={index} className="gap-2 flex flex-wrap items-center">
					<NativeSelect
						className="w-36"
						value={rule.dimension}
						options={DIMENSION_OPTIONS}
						onChange={(e) =>
							patchRule(index, { dimension: e.target.value as ContentRuleDimension })
						}
					/>
					<NativeSelect
						className="w-40"
						value={rule.mode}
						options={MODE_OPTIONS}
						onChange={(e) => patchRule(index, { mode: e.target.value as ContentRuleMode })}
					/>
					<Input
						variant="primary"
						containerClassName="w-72 max-w-full"
						placeholder="Values, comma separated"
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
							Hide items without a value
						</label>
					)}
					<Button
						size="icon"
						type="button"
						title="Remove rule"
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
					Add rule
				</Button>
				<Button type="button" disabled={isPending} onClick={handleSave}>
					Save restrictions
				</Button>
			</div>
		</div>
	)
}
