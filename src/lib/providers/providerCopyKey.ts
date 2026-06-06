/**
 * Generate a unique "-copy" suffixed provider key that does not collide with
 * any existing key. Extracted from App.tsx (L26 conservative split): pure
 * function, no React/component state.
 *
 * - `<originalKey>-copy` when free,
 * - otherwise the first free `<originalKey>-copy-<n>` starting at n = 2.
 */
export const generateUniqueProviderCopyKey = (
  originalKey: string,
  existingKeys: string[],
): string => {
  const baseKey = `${originalKey}-copy`;

  if (!existingKeys.includes(baseKey)) {
    return baseKey;
  }

  let counter = 2;
  while (existingKeys.includes(`${baseKey}-${counter}`)) {
    counter++;
  }
  return `${baseKey}-${counter}`;
};
