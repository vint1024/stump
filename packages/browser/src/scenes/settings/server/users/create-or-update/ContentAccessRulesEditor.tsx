import { useSDK, useSuspenseGraphQL } from '@stump/client'
import { Button, ComboBox, NativeSelect } from '@stump/components'
import { ContentRuleDimension, ContentRuleMode, graphql } from '@stump/graphql'
import { useLocaleContext } from '@stump/i18n'
import { Plus, X } from 'lucide-react'
import { useCallback, useMemo } from 'react'

const optionsQuery = graphql(`
	query ContentAccessRulesEditorOptions {
		tags {
			name
		}
		mediaMetadataOverview {
			genres
			publishers
		}
	}
`)

export type EditableRule = {
	dimension: ContentRuleDimension
	mode: ContentRuleMode
	/** The selected values (tag names / genres / publishers) for this rule */
	values: string[]
	restrictOnUnset: boolean
}

const LOCALE_BASE = 'settingsScene.server/users.createOrUpdateForm.contentRules'

type Props = {
	rules: EditableRule[]
	onChange: (rules: EditableRule[]) => void
}

/**
 * Controlled editor for a user's content access rules. Holds no state of its
 * own — the parent owns the `rules` array. Values are picked from a searchable
 * dropdown sourced per dimension (tags / genres / publishers); a value not yet
 * in the list can still be typed and added (e.g. a tag no book carries yet).
 */
export default function ContentAccessRulesEditor({ rules, onChange }: Props) {
	const { sdk } = useSDK()
	const { t } = useLocaleContext()

	const {
		data: { tags, mediaMetadataOverview },
	} = useSuspenseGraphQL(optionsQuery, sdk.cacheKey('tags', ['contentRuleOptions']))

	const sourceByDimension = useMemo<Record<ContentRuleDimension, string[]>>(
		() => ({
			[ContentRuleDimension.Tag]: tags.map((tag) => tag.name),
			[ContentRuleDimension.Genre]: mediaMetadataOverview.genres,
			[ContentRuleDimension.Publisher]: mediaMetadataOverview.publishers,
		}),
		[tags, mediaMetadataOverview],
	)

	const dimensionOptions = [
		{ label: t(`${LOCALE_BASE}.dimensions.tag`), value: ContentRuleDimension.Tag },
		{ label: t(`${LOCALE_BASE}.dimensions.publisher`), value: ContentRuleDimension.Publisher },
		{ label: t(`${LOCALE_BASE}.dimensions.genre`), value: ContentRuleDimension.Genre },
	]
	const modeOptions = [
		{ label: t(`${LOCALE_BASE}.modes.exclude`), value: ContentRuleMode.Exclude },
		{ label: t(`${LOCALE_BASE}.modes.only`), value: ContentRuleMode.Only },
	]

	const patchRule = useCallback(
		(index: number, patch: Partial<EditableRule>) =>
			onChange(rules.map((rule, i) => (i === index ? { ...rule, ...patch } : rule))),
		[rules, onChange],
	)

	// Merge the rule's currently-selected values into the option list so custom
	// values (typed in, not from the source) still render with a label
	const optionsForRule = useCallback(
		(rule: EditableRule) => {
			const source = sourceByDimension[rule.dimension] ?? []
			return Array.from(new Set([...source, ...rule.values]))
				.map((value) => ({ label: value, value }))
				.sort((a, b) => a.label.localeCompare(b.label))
		},
		[sourceByDimension],
	)

	return (
		<div className="gap-y-4 flex flex-col">
			{rules.map((rule, index) => (
				<div key={index} className="gap-2 flex flex-wrap items-center">
					<NativeSelect
						className="w-36"
						value={rule.dimension}
						options={dimensionOptions}
						// Switching dimension clears the values (a tag isn't a valid genre)
						onChange={(e) =>
							patchRule(index, {
								dimension: e.target.value as ContentRuleDimension,
								values: [],
							})
						}
					/>
					<NativeSelect
						className="w-40"
						value={rule.mode}
						options={modeOptions}
						// restrictOnUnset only applies to "Only" mode — clear it when
						// switching to "Exclude" so a stale true isn't carried along
						onChange={(e) => {
							const mode = e.target.value as ContentRuleMode
							patchRule(
								index,
								mode === ContentRuleMode.Exclude ? { mode, restrictOnUnset: false } : { mode },
							)
						}}
					/>
					<ComboBox
						isMultiSelect
						filterable
						size="md"
						value={rule.values}
						options={optionsForRule(rule)}
						placeholder={t(`${LOCALE_BASE}.valuesPlaceholder`)}
						filterPlaceholder={t(`${LOCALE_BASE}.valuesPlaceholder`)}
						// Enables the "Add" button for a typed value not in the list;
						// the actual append happens through onChange
						onAddOption={() => {}}
						onChange={(values) => patchRule(index, { values: values ?? [] })}
					/>
					{rule.mode === ContentRuleMode.Only && (
						<label className="gap-1.5 text-sm flex cursor-pointer items-center text-muted-foreground">
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
						onClick={() => onChange(rules.filter((_, i) => i !== index))}
					>
						<X className="h-4 w-4 text-muted-foreground" />
					</Button>
				</div>
			))}

			<div>
				<Button
					type="button"
					variant="outline"
					size="sm"
					onClick={() =>
						onChange([
							...rules,
							{
								dimension: ContentRuleDimension.Tag,
								mode: ContentRuleMode.Exclude,
								values: [],
								restrictOnUnset: false,
							},
						])
					}
				>
					<Plus className="mr-2 h-4 w-4" />
					{t(`${LOCALE_BASE}.addRule`)}
				</Button>
			</div>
		</div>
	)
}
