// Integration Test for Full Flutter App Navigation & UI Initialization Workflow

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:uot_app/main.dart';
import 'package:uot_app/src/rust/frb_generated.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(() async {
    await RustLib.init();
  });

  testWidgets('Full Flutter UI initialization and navigation smoke test',
      (WidgetTester tester) async {
    await tester.pumpWidget(const MyApp());
    await tester.pumpAndSettle();

    // Verify main navigation bar elements
    expect(find.text('Nearby'), findsWidgets);

    // Verify My Device Banner or Quick Actions present
    expect(find.byType(Card), findsWidgets);
  });
}
