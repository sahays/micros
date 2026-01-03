---
name: flutter-forms
description: Build forms with validation, state management, and user input handling in Flutter. Use when creating login forms, settings, user profiles, or any data collection interface.
---

# Flutter Forms

## Core Principles

**Form widget for grouping**: Wrap related fields in Form widget for unified validation and state management.

**GlobalKey for access**: Use `GlobalKey<FormState>` to trigger validation and save from anywhere.

**Validation on submit**: Validate when user submits, not on every keystroke (unless specifically required).

**Clear error messages**: Provide specific, actionable feedback for validation failures.

## Basic Form Structure

**Form with key**: Create Form widget with GlobalKey<FormState>, add TextFormField children.

**Validation method**: Call `formKey.currentState.validate()` to run all validators, returns bool.

**Save method**: Call `formKey.currentState.save()` to trigger all onSaved callbacks.

**Reset method**: Call `formKey.currentState.reset()` to clear all fields and errors.

## TextFormField Validation

**Validator function**: Return error string on failure, null on success.

**Common patterns**: Required field (check empty), email (regex), min length, numeric (tryParse), password strength.

**Email regex**: Use pattern like `^[\w-\.]+@([\w-]+\.)+[\w-]{2,4}$` for basic validation.

**Combine checks**: Validate multiple conditions, return first error encountered.

## Validation Modes

**AutovalidateMode**: Control when validation runs - disabled, always, onUserInteraction.

**Best practice**: Start with disabled, enable onUserInteraction after first submit attempt.

**Dynamic switching**: Change mode in setState after validation failure to provide immediate feedback.

## Saving Form Data

**onSaved callback**: Extract field values when form is saved, store in state variables.

**TextEditingController**: Alternative approach for programmatic access to field values.

**Controller lifecycle**: Initialize controllers, access via `.text`, dispose in dispose() method.

**Mixed approach**: Use onSaved for simple forms, controllers for complex logic.

## Input Types

**Keyboard types**: Set `keyboardType` for appropriate keyboard - emailAddress, number, phone, url.

**Text input actions**: Use `textInputAction` - next (move to next field), done (submit form).

**Obscure text**: Enable for password fields to hide input characters.

**Max length**: Set character limit with optional counter display.

## Dropdowns and Pickers

**DropdownButtonFormField**: Dropdown integrated with Form validation.

**Items**: Provide list of DropdownMenuItem widgets with value and child.

**Validation**: Check if value is null for required dropdown fields.

**Date/time pickers**: Show dialog with `showDatePicker` or `showTimePicker`, update TextFormField controller.

**Read-only fields**: Set `readOnly: true` on TextFormField for picker-based inputs.

## Checkboxes and Switches

**FormField wrapper**: Use FormField<bool> to integrate checkboxes/switches with Form validation.

**Builder pattern**: Use FormField builder to access state and display errors.

**State management**: Call `state.didChange(value)` on checkbox change.

**Error display**: Check `state.hasError` and show `state.errorText` conditionally.

## Form State Management

**Local state**: Use StatefulWidget for simple forms with few fields.

**Provider/Riverpod**: Extract form logic to separate class for complex multi-screen forms.

**Bloc/Cubit**: Event-driven validation for multi-step or async validation workflows.

**Form models**: Create model class with validation methods for reusable form logic.

## Async Validation

**Async validator**: Return Future<String?> from validator for backend checks.

**Debouncing**: Delay validation to avoid excessive API calls (use debounce packages).

**Loading states**: Show progress indicator during async validation.

**Error handling**: Handle network errors gracefully, provide fallback validation.

## Common Patterns

**Multi-step forms**: Use PageView with Form validation per step/page.

**Dynamic fields**: Use ListView.builder with list of controllers and validators.

**Focus management**: Use FocusNode to control keyboard focus programmatically.

**Submit loading**: Disable submit button and show loading indicator during async submit.

**Success feedback**: Show SnackBar confirmation or navigate to success screen.

**Error handling**: Display general form errors (network, server) above submit button.

## Accessibility

**Field labels**: Provide clear labelText in InputDecoration for every field.

**Hint text**: Add hintText for format examples or placeholder guidance.

**Error announcements**: Screen readers automatically announce validation errors.

**Focus order**: Ensure logical tab order, use FocusNode to customize if needed.

## Performance

**Controller reuse**: Create controllers in initState, reuse in build, dispose in dispose.

**Pure validators**: Keep validation functions pure and fast, avoid heavy computation.

**Selective rebuilds**: Use const widgets and ValueListenableBuilder to minimize rebuilds.

**Dispose properly**: Always dispose controllers and focus nodes to prevent memory leaks.
