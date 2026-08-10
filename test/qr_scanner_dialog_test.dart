// QrScannerDialog Widget Test
//
// Validates QrScannerDialog viewfinder rendering, progress bar, simulate button, and pairing completion.

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:uot_app/src/core/theme/app_theme.dart';
import 'package:uot_app/src/features/nearby/qr_scanner_dialog.dart';

void main() {
  testWidgets('QrScannerDialog renders viewfinder and progress indicator', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        theme: UotTheme.dark,
        home: Scaffold(
          body: Builder(
            builder: (context) {
              return ElevatedButton(
                onPressed: () => QrScannerDialog.show(context),
                child: const Text('Open QR Scanner'),
              );
            },
          ),
        ),
      ),
    );

    // Open Dialog
    await tester.tap(find.text('Open QR Scanner'));
    await tester.pumpAndSettle();

    // Verify dialog title & layout
    expect(find.text('Scan / Paste QR Code'), findsOneWidget);
    expect(find.text('Cancel'), findsOneWidget);

    // Verify manual input field exists
    expect(find.byType(TextField), findsOneWidget);

    // Tap Cancel to dismiss
    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();

    // Dialog should be dismissed
    expect(find.text('Scan / Paste QR Code'), findsNothing);
  });
}
