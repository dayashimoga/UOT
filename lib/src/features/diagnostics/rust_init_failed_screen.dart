// Rust Engine Initialization Failure Screen
//
// Displays when RustLib.init() fails, providing diagnostics
// and preventing users from attempting transfers with a broken engine.

import 'dart:io' show Platform;
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

/// Diagnostic screen shown when the Rust FFI engine fails to initialize.
///
/// Prevents users from accessing transfer functionality with a broken engine
/// and provides actionable diagnostics for bug reporting.
class RustInitFailedScreen extends StatelessWidget {
  const RustInitFailedScreen({
    super.key,
    required this.error,
    required this.stackTrace,
    required this.onRetry,
  });

  /// The error that caused initialization failure.
  final Object error;

  /// Stack trace from the initialization failure.
  final StackTrace stackTrace;

  /// Callback to retry initialization.
  final VoidCallback onRetry;

  String _buildDiagnostics() {
    final buffer = StringBuffer();
    buffer.writeln('=== UOT Engine Initialization Failure ===');
    buffer.writeln('');
    buffer.writeln('Error: $error');
    buffer.writeln('');
    buffer.writeln('Platform: ${Platform.operatingSystem}');
    buffer.writeln('OS Version: ${Platform.operatingSystemVersion}');
    buffer.writeln('Dart Version: ${Platform.version}');
    buffer.writeln('');
    buffer.writeln('Stack Trace:');
    buffer.writeln('$stackTrace');
    return buffer.toString();
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Scaffold(
      body: SafeArea(
        child: Center(
          child: SingleChildScrollView(
            padding: const EdgeInsets.all(24),
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                Icon(
                  Icons.warning_amber_rounded,
                  size: 72,
                  color: colorScheme.error,
                ),
                const SizedBox(height: 24),
                Text(
                  'Engine Initialization Failed',
                  style: Theme.of(context).textTheme.headlineSmall?.copyWith(
                        color: colorScheme.error,
                        fontWeight: FontWeight.bold,
                      ),
                  textAlign: TextAlign.center,
                ),
                const SizedBox(height: 12),
                Text(
                  'The native transfer engine could not be loaded. '
                  'File transfers and device discovery are unavailable.',
                  style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                        color: colorScheme.onSurfaceVariant,
                      ),
                  textAlign: TextAlign.center,
                ),
                const SizedBox(height: 24),
                // Error details card
                Card(
                  color: colorScheme.errorContainer.withValues(alpha: 0.3),
                  child: Padding(
                    padding: const EdgeInsets.all(16),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          'Error Details',
                          style:
                              Theme.of(context).textTheme.titleSmall?.copyWith(
                                    fontWeight: FontWeight.bold,
                                  ),
                        ),
                        const SizedBox(height: 8),
                        SelectableText(
                          '$error',
                          style:
                              Theme.of(context).textTheme.bodySmall?.copyWith(
                                    fontFamily: 'monospace',
                                    color: colorScheme.error,
                                  ),
                        ),
                      ],
                    ),
                  ),
                ),
                const SizedBox(height: 24),
                // Action buttons
                FilledButton.icon(
                  onPressed: onRetry,
                  icon: const Icon(Icons.refresh_rounded),
                  label: const Text('Retry Initialization'),
                ),
                const SizedBox(height: 12),
                OutlinedButton.icon(
                  onPressed: () {
                    final diagnostics = _buildDiagnostics();
                    Clipboard.setData(ClipboardData(text: diagnostics));
                    ScaffoldMessenger.of(context).showSnackBar(
                      const SnackBar(
                        content: Text('Diagnostics copied to clipboard'),
                      ),
                    );
                  },
                  icon: const Icon(Icons.copy_rounded),
                  label: const Text('Copy Diagnostics'),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
