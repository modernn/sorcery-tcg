import tseslint from 'typescript-eslint';

export default tseslint.config({
  files: ['src/**/*.ts', 'tests/**/*.ts', 'scripts/**/*.ts'],
  extends: [tseslint.configs.recommended],
});
