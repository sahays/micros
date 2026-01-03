---
name: react-forms
description:
  Build performant, accessible React forms using modern patterns. Use when implementing forms with validation, error
  handling, and submission. Focuses on React Hook Form, Zod validation, and Shadcn form components.
---

# React Forms

## Modern Form Stack

**React Hook Form**: Uncontrolled forms for better performance. Minimal re-renders.

**Zod**: Type-safe schema validation. Runtime validation with TypeScript types.

**Shadcn Form components**: Pre-built form components using React Hook Form and Radix UI. Accessible by default.

**Pattern**: Shadcn Form + React Hook Form + Zod = Modern, performant, accessible forms.

## React Hook Form Basics

**Setup**:
- `useForm()` hook to initialize form
- `register()` to register inputs
- `handleSubmit()` for form submission
- Minimal re-renders, uncontrolled by default

**Form modes**:
- `onSubmit`: Validate on submit (default, best performance)
- `onChange`: Validate on every change (immediate feedback)
- `onBlur`: Validate when field loses focus (balanced)
- `onTouched`: Validate after first blur, then on change

**Prefer onSubmit or onBlur**: Less aggressive validation, better UX.

## Zod Schema Validation

**Define schema with Zod**:
- Type-safe validation rules
- Automatic TypeScript types from schema
- Reusable schemas
- Custom error messages

**Integrate with React Hook Form**: Use `@hookform/resolvers/zod` to connect Zod schema to form.

**Server-side validation**: Use same Zod schema on backend for consistency.

**Benefits**: Single source of truth for validation, type safety, clear error messages.

## Shadcn Form Components

**Use Shadcn Form components**: Pre-built, accessible, composable.

**Core components**:
- `Form`: Root wrapper with FormProvider
- `FormField`: Connects field to form state
- `FormItem`: Field container with spacing
- `FormLabel`: Accessible label
- `FormControl`: Input wrapper
- `FormDescription`: Help text
- `FormMessage`: Error message display

**Pattern**: Consistent structure across all form fields.

## Form State Management

**Field registration**: Register inputs with `register()` or `Controller` for custom components.

**Watch values**: Use `watch()` to observe field values for conditional logic.

**Set values programmatically**: Use `setValue()` to update fields.

**Reset form**: Use `reset()` to clear or restore default values.

**Form errors**: Access via `formState.errors`.

**Dirty/touched state**: Track user interaction with `formState.isDirty` and `formState.touchedFields`.

## Validation Patterns

**Required fields**: Zod `.min(1)` for strings, `.refine()` for custom rules.

**Email validation**: Zod `.email()` with custom error message.

**Password strength**: Use `.refine()` with custom validation function.

**Conditional validation**: Use Zod `.refine()` or `.superRefine()` for cross-field validation.

**Async validation**: React Hook Form supports async validation (check username availability).

**Client + server validation**: Always validate on server. Client validation is UX enhancement.

## Error Handling

**Display errors**: Use `FormMessage` component or `formState.errors`.

**Error timing**: Show errors after user interaction (touched/blur), not immediately.

**Inline errors**: Display errors near the field, not just at form level.

**Error messages**: Clear, actionable, specific. "Email is required" not "Invalid input".

**Field-level errors**: For individual field issues.

**Form-level errors**: For submit errors (API errors, network issues).

**Accessible errors**: Use `aria-invalid` and `aria-describedby`. Shadcn handles this automatically.

## Form Submission

**Submission pattern**:
- `handleSubmit(onValid, onInvalid)` wrapper
- `onValid`: Called with validated data when form is valid
- `onInvalid`: Optional callback for validation errors

**Loading states**: Disable submit button during submission. Show loading indicator.

**Success handling**: Show success message, redirect, or reset form.

**Error handling**: Display server errors clearly. Map backend errors to form fields when possible.

**Optimistic updates**: Update UI immediately, rollback on error (with TanStack Query).

## Accessibility

**Labels**: Every input needs associated label. Use `FormLabel` component.

**Error announcements**: Screen readers announce errors. Use `aria-live` for dynamic errors.

**Focus management**: Focus first error field on validation failure.

**Keyboard navigation**: All fields keyboard accessible. Tab order logical.

**Required indicators**: Visual indicator (asterisk) and `aria-required`.

**Help text**: Use `FormDescription` for additional context.

**Field groups**: Use `fieldset` and `legend` for radio/checkbox groups.

## Performance Optimization

**Uncontrolled inputs**: React Hook Form uses uncontrolled inputs by default. Fewer re-renders.

**Isolated re-renders**: Only re-render components that need updates.

**Lazy validation**: Validate on blur/submit, not on every keystroke (unless needed).

**Avoid watching all fields**: Watch only fields you need for conditional logic.

**Debounce async validation**: Debounce server-side validation (username check, email verification).

**Large forms**: Use `shouldUnregister: false` and conditionally render sections.

## Multi-Step Forms

**Approach 1 - Single form**: Keep all data in one form, conditionally render steps.

**Approach 2 - Multiple forms**: Separate form per step, store data in Zustand between steps.

**Navigation**: Validate current step before allowing next step.

**Progress indicator**: Show current step and total steps.

**Back button**: Allow going back without losing data.

**Auto-save drafts**: Save progress to localStorage or backend.

## Dynamic Fields

**Field arrays**: Use `useFieldArray()` for dynamic lists (add/remove items).

**Add field**: `append()` to add new field.

**Remove field**: `remove(index)` to delete field.

**Unique keys**: Use `field.id` from useFieldArray for stable keys.

**Validation**: Validate entire array with Zod array schema.

## File Uploads

**File inputs**: Use `register()` with `type="file"`.

**File validation**: Validate file type and size with Zod or custom validation.

**Preview**: Show image preview or file name after selection.

**Upload strategy**:
- Client upload: Upload to cloud storage (S3, Cloudinary) directly from client
- Server upload: Send file to backend, backend uploads

**Large files**: Show progress indicator. Consider chunked uploads.

## Custom Components

**Controlled components**: Use `Controller` for custom inputs (date pickers, rich text, etc.).

**Third-party components**: Wrap with `Controller` to integrate with form.

**Custom validation**: Add validation via Zod schema or field rules.

## Common Patterns

**Search/filter forms**: Use `watch()` to trigger search on field changes. Debounce for API calls.

**Auto-save forms**: Use `watch()` with debounced save function.

**Confirmation dialogs**: Warn before leaving form with unsaved changes (`formState.isDirty`).

**Disabled state**: Disable submit while submitting or when form invalid.

**Reset after submit**: Call `reset()` after successful submission.

## Server-Side Integration

**API submission**: POST form data to backend endpoint.

**Validation errors from server**: Map backend errors to form fields using `setError()`.

**Error format**: Backend should return field-specific errors.

**Success response**: Return created resource or success message.

**TanStack Query mutation**: Use for form submission with automatic loading/error states.

## Form Testing

**Test user flows**: Fill form, submit, verify success/error states.

**Test validation**: Invalid inputs show errors, valid inputs submit.

**Accessibility testing**: Check labels, ARIA attributes, keyboard navigation.

**Mock API calls**: Use MSW to test submission success/failure.

**Don't test library internals**: Test form behavior, not React Hook Form implementation.

## Common Form Types

**Login form**: Email/username + password. Remember me checkbox optional.

**Registration form**: Email, password, password confirmation. Terms acceptance.

**Profile form**: Pre-fill from user data. Update specific fields.

**Search/filter form**: Real-time filtering with debounce. Clear all button.

**Payment form**: Card details with validation. Address fields.

**Multi-step wizard**: Progress indicator, navigation, data persistence.

## Anti-Patterns

**Avoid**:
- Controlled inputs everywhere (unnecessary re-renders)
- Validating on every keystroke (aggressive, poor UX)
- Client-side validation only (security risk)
- Generic error messages ("Invalid input")
- Submitting without loading state
- Not disabling submit during submission
- Losing form data on navigation
- Mixing controlled and uncontrolled

**Do**:
- Use uncontrolled inputs (React Hook Form default)
- Validate on blur or submit
- Validate on both client and server
- Specific, helpful error messages
- Show loading state during submission
- Disable submit button when loading
- Preserve unsaved changes or warn user
- Consistent controlled/uncontrolled pattern

## Production Checklist

**Before deploying forms**:
- All fields have labels
- Error messages are clear and helpful
- Server-side validation matches client-side
- Loading states work correctly
- Success/error states tested
- Keyboard navigation works
- Screen reader tested
- Mobile responsive
- File uploads size limits enforced
- Rate limiting on submission
