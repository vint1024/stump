import { useLocaleContext } from '@stump/i18n'

import GenericFilterMultiselect from './GenericFilterMultiselect'

export default function ReadStatusSelect() {
	const { t } = useLocaleContext()

	return (
		<GenericFilterMultiselect
			name="read_status"
			label={t('components.filters.form.ReadStatusSelect.label')}
			options={[
				{
					label: t('components.filters.form.ReadStatusSelect.completed'),
					value: 'finished',
				},
				{
					label: t('components.filters.form.ReadStatusSelect.reading'),
					value: 'reading',
				},
				{
					label: t('components.filters.form.ReadStatusSelect.unread'),
					value: 'not_started',
				},
			]}
		/>
	)
}
