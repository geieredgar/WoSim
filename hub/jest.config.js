module.exports = {
  moduleNameMapper: {
    '^@/(.*)$': '<rootDir>/$1',
    '^~/(.*)$': '<rootDir>/$1',
    '^vue$': 'vue/dist/vue.common.js',
    '^@tauri-apps/api/http$': '<rootDir>/tests/mocks/tauri.ts',
  },
  moduleFileExtensions: ['ts', 'js', 'vue', 'json'],
  transformIgnorePatterns: ['<rootDir>/node_modules/@tauri-apps'],
  transform: {
    '^.+\\.ts$': 'ts-jest',
    '^.+\\.js$': 'babel-jest',
    '.*\\.(vue)$': 'vue-jest',
  },
  collectCoverage: true,
  collectCoverageFrom: [
    '<rootDir>/components/**/*.vue',
    '<rootDir>/pages/**/*.vue',
  ],
  setupFiles: ['<rootDir>/tests/setup.ts'],
}
