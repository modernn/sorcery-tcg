import tseslint from 'typescript-eslint';

export default tseslint.config(
  {
    ignores: ['.local/**', '.worktrees/**', 'no/**', 'target/**', 'scripts/__pycache__/**'],
  },
  {
    files: ['src/**/*.ts', 'tests/**/*.ts', 'scripts/**/*.ts'],
    extends: [tseslint.configs.recommended],
    languageOptions: { parserOptions: { tsconfigRootDir: import.meta.dirname } },
  },
);
