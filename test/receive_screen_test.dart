// Receive Screen Widget Test
//
// Validates Material 3 rendering, visibility card, receive settings, and empty state.

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:uot_app/src/core/theme/app_theme.dart';
import 'package:uot_app/src/features/receive/receive_screen.dart';

void main() {
  testWidgets('ReceiveScreen renders visibility card and settings section',
      (WidgetTester tester) async {
    await tester.pumpWidget(
      MaterialApp(
        theme: UotTheme.dark,
        home: const Scaffold(
          body: ReceiveScreen(),
        ),
      ),
    );

    // Verify Title and Subheaders
    expect(find.text('Receive'), findsOneWidget);
    expect(find.text('RECEIVE SETTINGS'), findsOneWidget);
    expect(find.text('INCOMING REQUESTS'), findsOneWidget);

    // Verify Visibility Toggle Text
    expect(find.text('Visible to nearby devices'), findsOneWidget);
    expect(find.text('Other devices can find you and send files'), findsOneWidget);

    // Verify Settings List Items
    expect(find.text('Auto-accept from trusted'), findsOneWidget);
    expect(find.text('Require PIN'), findsOneWidget);
    expect(find.text('Save location'), findsOneWidget);

    // Verify Empty State when no incoming requests
    expect(find.text('No incoming requests'), findsOneWidget);
  });
}
