import * as v from 'valibot'

/**
 * Shared validation schemas for auth endpoints.
 *
 * Placed in `shared/` so the same rules apply on both sides:
 * - Server routes use them to validate request bodies.
 * - Client pages (login.vue, register.vue) import the same objects for UForm.
 *
 * This is Nuxt 4's `shared/` layer — files here are auto-imported in both
 * `app/` and `server/` without any explicit import.
 */

export const loginSchema = v.object({
  email: v.pipe(v.string(), v.email('Enter a valid email address')),
  password: v.pipe(v.string(), v.minLength(8, 'At least 8 characters'))
})

export type LoginInput = v.InferOutput<typeof loginSchema>

export const registerSchema = v.pipe(
  v.object({
    name: v.pipe(v.string(), v.minLength(1, 'Name is required'), v.maxLength(100, 'Name too long')),
    email: v.pipe(v.string(), v.email('Enter a valid email address')),
    password: v.pipe(v.string(), v.minLength(8, 'At least 8 characters'), v.maxLength(128, 'Password too long')),
    confirm: v.string()
  }),
  v.forward(
    v.check(input => input.password === input.confirm, 'Passwords do not match'),
    ['confirm']
  )
)

export type RegisterInput = v.InferOutput<typeof registerSchema>

export const forgotPasswordSchema = v.object({
  email: v.pipe(v.string(), v.email('Enter a valid email address'))
})

export type ForgotPasswordInput = v.InferOutput<typeof forgotPasswordSchema>

export const resetPasswordSchema = v.pipe(
  v.object({
    token: v.pipe(v.string(), v.minLength(1, 'Token is required')),
    password: v.pipe(v.string(), v.minLength(8, 'At least 8 characters'), v.maxLength(128, 'Password too long')),
    confirm: v.string()
  }),
  v.forward(
    v.check(input => input.password === input.confirm, 'Passwords do not match'),
    ['confirm']
  )
)

export type ResetPasswordInput = v.InferOutput<typeof resetPasswordSchema>

export const verifySchema = v.object({
  token: v.pipe(v.string(), v.minLength(1, 'Token is required'), v.maxLength(256, 'Token is invalid'))
})

export type VerifyInput = v.InferOutput<typeof verifySchema>

export const reauthSchema = v.object({
  password: v.pipe(v.string(), v.minLength(8, 'Password is required'), v.maxLength(128, 'Password is too long'))
})

export const emailChangeSchema = v.object({
  email: v.pipe(v.string(), v.email('Enter a valid email address'), v.maxLength(320, 'Email is too long')),
  password: v.pipe(v.string(), v.minLength(8, 'Current password is required'), v.maxLength(128, 'Password is too long'))
})

export const mfaEnrollmentSchema = v.object({
  factorId: v.pipe(v.string(), v.uuid('Factor id is invalid')),
  code: v.pipe(v.string(), v.regex(/^\d{6}$/, 'Enter the six-digit code'))
})

export const mfaFactorSchema = v.object({
  factorId: v.pipe(v.string(), v.uuid('Factor id is invalid'))
})

export const recoveryCodeSchema = v.object({
  code: v.pipe(v.string(), v.regex(/^[a-f0-9]{4}(?:-[a-f0-9]{4}){3}$/i, 'Recovery code is invalid'))
})
