export default {
  extends: ['@commitlint/config-conventional'],
  rules: {
    'type-enum': [
      2,
      'always',
      [
        'feat',
        'fix',
        'docs',
        'style',
        'refactor',
        'perf',
        'test',
        'build',
        'ci',
        'chore',
        'security',
      ],
    ],
    'scope-enum': [
      1,
      'always',
      ['core', 'ui', 'cli', 'extension', 'sync', 'deps'],
    ],
    'subject-case': [0],
    'body-max-line-length': [1, 'always', 100],
  },
};
