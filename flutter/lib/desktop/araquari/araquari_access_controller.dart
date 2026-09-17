import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:uuid/uuid.dart';

import 'araquari_auth_models.dart';
import 'araquari_auth_service.dart';

enum AraquariAccessStatus { signedOut, authenticating, authenticated }

class AraquariAuthAttempt {
  const AraquariAuthAttempt.success()
      : authenticated = true,
        errorKind = null;

  const AraquariAuthAttempt.failure(this.errorKind) : authenticated = false;

  final bool authenticated;
  final AraquariAuthErrorKind? errorKind;
}

/// Central authority for the TI session.
///
/// Tokens exist only in memory. Closing the process always returns the client
/// to User Mode, and every outgoing connection validates the server session.
class AraquariAccessController extends ChangeNotifier {
  AraquariAccessController({AraquariAuthService? authService})
      : _authService = authService ?? AraquariAuthService();

  static final instance = AraquariAccessController();

  final AraquariAuthService _authService;
  AraquariAccessStatus _status = AraquariAccessStatus.signedOut;
  AraquariSession? _session;
  String? _auditWarning;

  AraquariAccessStatus get status => _status;
  bool get isAuthenticating =>
      _status == AraquariAccessStatus.authenticating;
  bool get isTiMode =>
      _status == AraquariAccessStatus.authenticated &&
      _session != null &&
      !_session!.isExpired;
  AraquariUser? get currentUser => isTiMode ? _session!.user : null;
  String? get auditWarning => _auditWarning;

  Future<AraquariAuthAttempt> authenticate(
    String username,
    String password,
  ) async {
    if (_status == AraquariAccessStatus.authenticating) {
      return const AraquariAuthAttempt.failure(
        AraquariAuthErrorKind.serviceUnavailable,
      );
    }
    _status = AraquariAccessStatus.authenticating;
    notifyListeners();
    try {
      final session = await _authService.login(username, password);
      _session = session;
      _status = AraquariAccessStatus.authenticated;
      _auditWarning = null;
      notifyListeners();
      unawaited(_sendAudit(eventType: 'TI_MODE_STARTED', result: 'SUCCESS'));
      return const AraquariAuthAttempt.success();
    } on AraquariAuthException catch (error) {
      _session = null;
      _status = AraquariAccessStatus.signedOut;
      notifyListeners();
      return AraquariAuthAttempt.failure(error.kind);
    }
  }

  Future<bool> ensureValidSession() async {
    final session = _session;
    if (!isTiMode || session == null) {
      await _clearLocalSession();
      return false;
    }
    final valid = await _authService.validate(session);
    if (!valid) await _clearLocalSession();
    return valid;
  }

  Future<String?> authorizeRemoteConnection(String targetRustDeskId) async {
    if (!await ensureValidSession()) return null;
    final supportSessionId = Uuid().v4();
    final audited = await _sendAudit(
      eventType: 'REMOTE_CONNECTION_REQUESTED',
      supportSessionId: supportSessionId,
      clientDevice: AraquariDeviceIdentity(rustDeskId: targetRustDeskId),
      result: 'REQUESTED',
    );
    if (!audited) {
      debugPrint('AraquariDesk: remote request continued with audit warning');
    }
    return supportSessionId;
  }

  Future<void> remoteConnectionFailed(
    String supportSessionId,
    String targetRustDeskId,
  ) async {
    await _sendAudit(
      eventType: 'REMOTE_CONNECTION_FAILED',
      supportSessionId: supportSessionId,
      clientDevice: AraquariDeviceIdentity(rustDeskId: targetRustDeskId),
      result: 'FAILED_TO_OPEN',
    );
  }

  Future<void> logout() async {
    final session = _session;
    await _clearLocalSession();
    if (session != null && !session.isExpired) {
      try {
        await _authService.audit(
          session,
          eventType: 'TI_MODE_ENDED',
          result: 'SUCCESS',
        );
      } catch (error) {
        debugPrint('AraquariDesk TI logout audit error: $error');
      }
      await _authService.logout(session);
    }
  }

  Future<bool> _sendAudit({
    required String eventType,
    String? supportSessionId,
    AraquariDeviceIdentity? clientDevice,
    String? result,
  }) async {
    final session = _session;
    if (session == null || session.isExpired) return false;
    try {
      await _authService.audit(
        session,
        eventType: eventType,
        supportSessionId: supportSessionId,
        clientDevice: clientDevice,
        result: result,
      );
      if (_auditWarning != null) {
        _auditWarning = null;
        notifyListeners();
      }
      return true;
    } catch (error) {
      debugPrint('AraquariDesk audit error: $error');
      _auditWarning = 'Falha ao registrar auditoria no servidor.';
      notifyListeners();
      return false;
    }
  }

  Future<void> _clearLocalSession() async {
    if (_session == null && _status == AraquariAccessStatus.signedOut) return;
    _session = null;
    _status = AraquariAccessStatus.signedOut;
    _auditWarning = null;
    notifyListeners();
  }
}
