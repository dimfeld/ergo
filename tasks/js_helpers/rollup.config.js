import nodeResolve from '@rollup/plugin-node-resolve';
import terser from '@rollup/plugin-terser';
import typescript from '@rollup/plugin-typescript';

export default [
  { input: 'src/dataflow.ts',
    output: { file: 'rust-dist/dataflow.js', name: '__ergo_dataflow', format: 'iife' },
    plugins: [
      nodeResolve(),
      typescript({
        tsconfig: false,
        compilerOptions: {
          lib: ['DOM', 'ES2020'],
          target: 'ES2020'
        }
      }),
      process.env.NODE_ENV === 'production' && terser(),
    ]
  }
]
