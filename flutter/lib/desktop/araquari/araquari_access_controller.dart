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
  String? _lastAuthorizationError;

  AraquariAccessStatus get status => _status;
  bool get isAuthenticating =>
      _status == AraquariAccessStatus.authenticating;
  bool get isTiMode =>
      _status == AraquariAccessStatus.authenticated &&
      _session != null &&
      !_session!.isExpired;
  AraquariUser? get currentUser => isTiMode ? _session!.user : null;
  String? get auditWarning => _auditWarning;
  String? get lastAuthorizationError => _lastAuthorizationError;

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
      _lastAuthorizationError = null;
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
    if (session == null || _status != AraquariAccessStatus.authenticated) {
      _setAuthorizationError(
        'Entre no Modo TI para iniciar uma conexão remota.',
      );
      await _clearLocalSession(clearAuthorizationError: false);
      return false;
    }
    if (session.isExpired) {
      _setAuthorizationError(
        'A sessão da TI expirou. Entre novamente para continuar.',
      );
      await _clearLocalSession(clearAuthorizationError: false);
      return false;
    }
    final valid = await _authService.validate(session);
    if (!valid) {
      _setAuthorizationError(
        'Não foi possível validar a sessão da TI no servidor.',
      );
      await _clearLocalSession(clearAuthorizationError: false);
    }
    return valid;
  }

  Future<String?> authorizeRemoteConnection(String targetRustDeskId) async {
    _setAuthorizationError(null);
    if (!await ensureValidSession()) return null;
    final supportSessionId = Uuid().v4();
    final audited = await _sendAudit(
      eventType: 'REMOTE_CONNECTION_REQUESTED',
      supportSessionId: supportSessionId,
      clientDevice: AraquariDeviceIdentity(rustDeskId: targetRustDeskId),
      result: 'REQUESTED',
    );
    if (!audited) {
      _setAuthorizationError(
        'A conexão foi bloqueada porque a auditoria está indisponível.',
      );
      return null;
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

  void _setAuthorizationError(String? value) {
    if (_lastAuthorizationError == value) return;
    _lastAuthorizationError = value;
    notifyListeners();
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

  Future<void> _clearLocalSession({bool clearAuthorizationError = true}) async {
    if (_session == null && _status == AraquariAccessStatus.signedOut) return;
    _session = null;
    _status = AraquariAccessStatus.signedOut;
    _auditWarning = null;
    if (clearAuthorizationError) _lastAuthorizationError = null;
    notifyListeners();
  }
}
