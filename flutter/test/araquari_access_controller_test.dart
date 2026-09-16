import 'package:flutter_hbb/desktop/araquari/araquari_access_controller.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('TI mode starts disabled and only opens after successful verification',
      () {
    final controller = AraquariAccessController(
      verifier: (username, password) =>
          username == 'authorized-user' && password == 'valid-secret',
    );

    expect(controller.isTiMode, isFalse);
    expect(controller.authenticate('invalid-user', 'invalid-secret'), isFalse);
    expect(controller.isTiMode, isFalse);
    expect(controller.authenticate('authorized-user', 'valid-secret'), isTrue);
    expect(controller.isTiMode, isTrue);
  });

  test('logout immediately returns to user mode', () {
    final controller = AraquariAccessController(
      verifier: (username, password) => true,
    );

    controller.authenticate('any-user', 'any-secret');
    controller.logout();

    expect(controller.isTiMode, isFalse);
  });
}
