// IncomingOfferDialog Widget Test
//
// Validates offer details rendering, file listing, PIN field presence when requested, and dialog action callbacks.

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:uot_app/src/core/theme/app_theme.dart';
import 'package:uot_app/src/features/receive/incoming_offer_dialog.dart';

void main() {
  testWidgets(
    'IncomingOfferDialog renders device name, items, and action buttons',
    (WidgetTester tester) async {
      await tester.pumpWidget(
        MaterialApp(
          theme: UotTheme.dark,
          home: Scaffold(
            body: Builder(
              builder: (context) {
                return ElevatedButton(
                  onPressed: () {
                    IncomingOfferDialog.show(
                      context,
                      transferId: 'test-transfer-123',
                      fromDevice: 'Pixel 8 Pro',
                      items: ['photo.jpg', 'document.pdf'],
                      totalSize: 1024 * 1024 * 3, // 3 MB
                      requirePin: true,
                    );
                  },
                  child: const Text('Open Dialog'),
                );
              },
            ),
          ),
        ),
      );

      // Tap button to launch dialog
      await tester.tap(find.text('Open Dialog'));
      await tester.pumpAndSettle();

      // Verify dialog header & device info
      expect(find.text('Incoming Transfer Offer'), findsOneWidget);
      expect(find.text('Pixel 8 Pro'), findsOneWidget);
      expect(find.text('2 file(s) • 3.0 MB'), findsOneWidget);

      // Verify file list items
      expect(find.text('photo.jpg'), findsOneWidget);
      expect(find.text('document.pdf'), findsOneWidget);

      // Verify PIN requirement section
      expect(find.text('SECURITY PIN VERIFICATION'), findsOneWidget);
      expect(find.text('Enter Receiver PIN'), findsOneWidget);

      // Verify action buttons
      expect(find.text('Decline'), findsOneWidget);
      expect(find.text('Accept'), findsOneWidget);

      // Tap Decline to close
      await tester.tap(find.text('Decline'));
      await tester.pumpAndSettle();

      expect(find.text('Incoming Transfer Offer'), findsNothing);
    },
  );
}
