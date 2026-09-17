import 'package:flutter_hbb/desktop/araquari/araquari_access_controller.dart';
import 'package:flutter_hbb/desktop/araquari/araquari_auth_models.dart';
import 'package:flutter_hbb/desktop/araquari/araquari_auth_service.dart';
import 'package:flutter_test/flutter_test.dart';

class _FakeAuthService extends AraquariAuthService {
  _FakeAuthService({this.loginSucceeds = true, this.sessionValid = true});

  final bool loginSucceeds;
  bool sessionValid;
  int auditCalls = 0;
  bool logoutCalled = false;

  @override
  Future<AraquariSession> login(String username, String password) async {
    if (!loginSucceeds) {
      throw const AraquariAuthException(
        AraquariAuthErrorKind.invalidCredentials,
      );
    }
    return AraquariSession(
      accessToken: 'memory-only-token',
      expiresAt: DateTime.now().toUtc().add(const Duration(hours: 1)),
      user: const AraquariUser(
        id: 'user-id',
        username: 'technician',
        displayName: 'Técnico de Teste',
        role: AraquariRole.technician,
      ),
    );
  }

  @override
  Future<bool> validate(AraquariSession session) async => sessionValid;

  @override
  Future<void> audit(
    AraquariSession session, {
    required String eventType,
    String? supportSessionId,
    AraquariDeviceIdentity? clientDevice,
    String? result,
    int? durationSeconds,
    Map<String, dynamic> metadata = const {},
  }) async {
    auditCalls++;
  }

  @override
  Future<void> logout(AraquariSession session) async {
    logoutCalled = true;
  }
}

void main() {
  test('TI mode opens only after server authentication', () async {
    final failed = AraquariAccessController(
      authService: _FakeAuthService(loginSucceeds: false),
    );
    expect((await failed.authenticate('user', 'wrong')).authenticated, isFalse);
    expect(failed.isTiMode, isFalse);

    final controller = AraquariAccessController(
      authService: _FakeAuthService(),
    );
    expect(controller.isTiMode, isFalse);
    expect((await controller.authenticate('user', 'valid')).authenticated, isTrue);
    expect(controller.isTiMode, isTrue);
    expect(controller.currentUser?.displayName, 'Técnico de Teste');
  });

  test('remote control is blocked without a valid server session', () async {
    final service = _FakeAuthService();
    final controller = AraquariAccessController(authService: service);
    expect(await controller.authorizeRemoteConnection('123456789'), isNull);

    await controller.authenticate('user', 'valid');
    service.sessionValid = false;
    expect(await controller.authorizeRemoteConnection('123456789'), isNull);
    expect(controller.isTiMode, isFalse);
  });

  test('logout revokes remotely and immediately returns to user mode', () async {
    final service = _FakeAuthService();
    final controller = AraquariAccessController(authService: service);
    await controller.authenticate('user', 'valid');
    await controller.logout();
    expect(service.logoutCalled, isTrue);
    expect(controller.isTiMode, isFalse);
  });
}

