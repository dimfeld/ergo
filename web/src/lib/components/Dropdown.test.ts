/**
 * @jest-environment jsdom
 */
import '@testing-library/jest-dom/extend-expect';
import { render, fireEvent } from '@testing-library/svelte';
import DropdownWithMenu from './fixtures/DropdownWithMenu.svelte';

test('Shows the dropdown when clicked', async () => {
  const { getByText } = render(DropdownWithMenu, { label: 'Show Menu' });
  const button = getByText('Show Menu');
  expect(button).toBeInTheDocument();

  await fireEvent.click(button);

  expect(getByText('Dropdown contents')).toBeInTheDocument();
});
