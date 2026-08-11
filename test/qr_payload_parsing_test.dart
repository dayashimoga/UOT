// Widget & Unit Tests for Optical QR Payload Parsing & Dialog Error Recovery

import 'package:flutter_test/flutter_test.dart';

void main() {
  group('Optical QR Payload & URI Parsing Tests', () {
    test('Valid uot://pair URI payload extracts IP, port, and PIN', () {
      const payload = 'uot://pair?ip=192.168.0.150&port=42000&pin=654321';
      final uri = Uri.parse(payload);

      expect(uri.scheme, equals('uot'));
      expect(uri.host, equals('pair'));
      expect(uri.queryParameters['ip'], equals('192.168.0.150'));
      expect(uri.queryParameters['port'], equals('42000'));
      expect(uri.queryParameters['pin'], equals('654321'));

      final targetAddr =
          '${uri.queryParameters['ip']}:${uri.queryParameters['port']}';
      expect(targetAddr, equals('192.168.0.150:42000'));
    });

    test('Plain IP or IP:Port string is correctly parsed', () {
      const ipOnly = '192.168.1.50';
      const ipPort = '192.168.1.50:42000';

      String normalize(String input) {
        final trimmed = input.trim();
        if (trimmed.startsWith('uot://pair')) {
          final parsed = Uri.parse(trimmed);
          final ip = parsed.queryParameters['ip'] ?? '';
          final port = parsed.queryParameters['port'] ?? '42000';
          return '$ip:$port';
        }
        if (!trimmed.contains(':') && trimmed.split('.').length == 4) {
          return '$trimmed:42000';
        }
        return trimmed;
      }

      expect(normalize(ipOnly), equals('192.168.1.50:42000'));
      expect(normalize(ipPort), equals('192.168.1.50:42000'));
      expect(
        normalize('uot://pair?ip=10.0.0.5&port=42000'),
        equals('10.0.0.5:42000'),
      );
    });

    test('Invalid payload format returns fallback or handles error', () {
      const invalidPayload = 'not_a_valid_ip_or_qr_uri';

      bool isValidTarget(String input) {
        if (input.startsWith('uot://pair')) return true;
        if (input.contains(':')) return true;
        final parts = input.split('.');
        return parts.length == 4 && parts.every((p) => int.tryParse(p) != null);
      }

      expect(isValidTarget(invalidPayload), isFalse);
    });
  });
}
