import nodeResolve from '@rollup/plugin-node-resolve';
import terser from '@rollup/plugin-terser';
export default [
  { input: 'src/dataflow.js',
    output: { file: 'dist/dataflow.js', name: '__ergo_dataflow', format: 'iife' },
    plugins: [
      nodeResolve(),
      process.env.NODE_ENV === 'production' && terser(),
    ]
  },
]
