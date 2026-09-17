import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:http/http.dart' as http;

import 'araquari_api_config.dart';
import 'araquari_auth_models.dart';

class AraquariAuthService {
  AraquariAuthService({http.Client? client}) : _client = client ?? http.Client();

  final http.Client _client;
  static const _timeout = Duration(seconds: 10);

  Future<AraquariSession> login(String username, String password) async {
    try {
      final response = await _client
          .post(
            AraquariApiConfig.endpoint('/api/v1/auth/login'),
            headers: const {'Content-Type': 'application/json'},
            body: jsonEncode({
              'username': username.trim(),
              'password': password,
              'device': (await currentDeviceIdentity()).toJson(),
            }),
          )
          .timeout(_timeout);
      if (response.statusCode == 401 || response.statusCode == 403) {
        throw const AraquariAuthException(
          AraquariAuthErrorKind.invalidCredentials,
        );
      }
      if (response.statusCode == 429) {
        throw const AraquariAuthException(AraquariAuthErrorKind.rateLimited);
      }
      if (response.statusCode < 200 || response.statusCode >= 300) {
        throw const AraquariAuthException(
          AraquariAuthErrorKind.serviceUnavailable,
        );
      }
      return AraquariSession.fromJson(
        jsonDecode(response.body) as Map<String, dynamic>,
      );
    } on AraquariAuthException {
      rethrow;
    } on TimeoutException {
      throw const AraquariAuthException(
        AraquariAuthErrorKind.serviceUnavailable,
      );
    } on SocketException {
      throw const AraquariAuthException(
        AraquariAuthErrorKind.serviceUnavailable,
      );
    } on FormatException catch (error) {
      throw AraquariAuthException(
        AraquariAuthErrorKind.invalidResponse,
        error.toString(),
      );
    } catch (error) {
      debugPrint('AraquariDesk authentication error: $error');
      throw const AraquariAuthException(
        AraquariAuthErrorKind.serviceUnavailable,
      );
    }
  }

  Future<bool> validate(AraquariSession session) async {
    if (session.isExpired) return false;
    try {
      final response = await _client
          .get(
            AraquariApiConfig.endpoint('/api/v1/auth/me'),
            headers: _authorizedHeaders(session.accessToken),
          )
          .timeout(_timeout);
      return response.statusCode >= 200 && response.statusCode < 300;
    } catch (error) {
      debugPrint('AraquariDesk session validation error: $error');
      return false;
    }
  }

  Future<void> logout(AraquariSession session) async {
    try {
      await _client
          .post(
            AraquariApiConfig.endpoint('/api/v1/auth/logout'),
            headers: _authorizedHeaders(session.accessToken),
          )
          .timeout(_timeout);
    } catch (error) {
      debugPrint('AraquariDesk logout notification error: $error');
    }
  }

  Future<void> audit(
    AraquariSession session, {
    required String eventType,
    String? supportSessionId,
    AraquariDeviceIdentity? clientDevice,
    String? result,
    int? durationSeconds,
    Map<String, dynamic> metadata = const {},
  }) async {
    final response = await _client
        .post(
          AraquariApiConfig.endpoint('/api/v1/audit/events'),
          headers: _authorizedHeaders(session.accessToken),
          body: jsonEncode({
            'event_type': eventType,
            if (supportSessionId != null)
              'support_session_id': supportSessionId,
            'technician_device': (await currentDeviceIdentity()).toJson(),
            if (clientDevice != null) 'client_device': clientDevice.toJson(),
            if (result != null) 'result': result,
            if (durationSeconds != null) 'duration_seconds': durationSeconds,
            'metadata': metadata,
          }),
        )
        .timeout(_timeout);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw HttpException('audit service returned ${response.statusCode}');
    }
  }

  Map<String, String> _authorizedHeaders(String token) => {
        'Authorization': 'Bearer $token',
        'Content-Type': 'application/json',
      };

  static Future<AraquariDeviceIdentity> currentDeviceIdentity() async {
    String? localIp;
    try {
      final interfaces = await NetworkInterface.list(
        type: InternetAddressType.IPv4,
        includeLoopback: false,
      );
      for (final interface in interfaces) {
        for (final address in interface.addresses) {
          if (address.isLoopback || address.type != InternetAddressType.IPv4) {
            continue;
          }
          localIp = address.address;
          break;
        }
        if (localIp != null) break;
      }
    } catch (_) {}
    return AraquariDeviceIdentity(
      hostname: Platform.localHostname,
      osUsername:
          Platform.environment['USERNAME'] ?? Platform.environment['USER'],
      localIp: localIp,
    );
  }
}

