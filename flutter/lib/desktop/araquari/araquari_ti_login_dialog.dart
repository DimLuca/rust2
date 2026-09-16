import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hbb/common.dart';
import 'package:flutter_hbb/desktop/araquari/araquari_access_controller.dart';
import 'package:flutter_svg/flutter_svg.dart';

Future<bool> showAraquariTiLoginDialog(BuildContext context) async {
  final result = await showDialog<bool>(
    context: context,
    barrierDismissible: false,
    builder: (_) => const _AraquariTiLoginDialog(),
  );
  return result ?? false;
}

class _AraquariTiLoginDialog extends StatefulWidget {
  const _AraquariTiLoginDialog();

  @override
  State<_AraquariTiLoginDialog> createState() =>
      _AraquariTiLoginDialogState();
}

class _AraquariTiLoginDialogState extends State<_AraquariTiLoginDialog> {
  final _usernameController = TextEditingController();
  final _passwordController = TextEditingController();
  final _formKey = GlobalKey<FormState>();
  bool _passwordVisible = false;
  bool _hasError = false;

  @override
  void dispose() {
    _usernameController.dispose();
    _passwordController.dispose();
    super.dispose();
  }

  void _submit() {
    if (!_formKey.currentState!.validate()) return;
    final authenticated = AraquariAccessController.instance.authenticate(
      _usernameController.text,
      _passwordController.text,
    );
    _passwordController.clear();
    if (authenticated) {
      Navigator.of(context).pop(true);
    } else {
      setState(() => _hasError = true);
    }
  }

  @override
  Widget build(BuildContext context) {
    final dark = Theme.of(context).brightness == Brightness.dark;
    return AlertDialog(
      titlePadding: const EdgeInsets.fromLTRB(28, 26, 28, 4),
      contentPadding: const EdgeInsets.fromLTRB(28, 18, 28, 8),
      actionsPadding: const EdgeInsets.fromLTRB(28, 8, 28, 24),
      title: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SvgPicture.asset(
            'assets/araquari_desk.svg',
            height: 38,
            colorFilter: dark
                ? const ColorFilter.mode(Colors.white, BlendMode.srcIn)
                : null,
          ),
          const SizedBox(height: 20),
          const Text('Acesso restrito da TI'),
        ],
      ),
      content: SizedBox(
        width: 390,
        child: Form(
          key: _formKey,
          child: Shortcuts(
            shortcuts: const {
              SingleActivator(LogicalKeyboardKey.enter): ActivateIntent(),
            },
            child: Actions(
              actions: {
                ActivateIntent: CallbackAction<ActivateIntent>(
                  onInvoke: (_) {
                    _submit();
                    return null;
                  },
                ),
              },
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'Área destinada à equipe autorizada de Tecnologia da Informação.',
                    style: Theme.of(context).textTheme.bodyMedium,
                  ),
                  const SizedBox(height: 20),
                  TextFormField(
                    controller: _usernameController,
                    autofocus: true,
                    autofillHints: const [AutofillHints.username],
                    decoration: const InputDecoration(
                      labelText: 'Usuário',
                      prefixIcon: Icon(Icons.badge_outlined),
                    ),
                    validator: (value) => value == null || value.trim().isEmpty
                        ? 'Informe o usuário.'
                        : null,
                    onChanged: (_) {
                      if (_hasError) setState(() => _hasError = false);
                    },
                  ),
                  const SizedBox(height: 14),
                  TextFormField(
                    controller: _passwordController,
                    obscureText: !_passwordVisible,
                    enableSuggestions: false,
                    autocorrect: false,
                    autofillHints: const [AutofillHints.password],
                    decoration: InputDecoration(
                      labelText: 'Senha',
                      prefixIcon: const Icon(Icons.lock_outline),
                      suffixIcon: IconButton(
                        tooltip: _passwordVisible
                            ? 'Ocultar senha'
                            : 'Mostrar senha',
                        onPressed: () => setState(
                          () => _passwordVisible = !_passwordVisible,
                        ),
                        icon: Icon(
                          _passwordVisible
                              ? Icons.visibility_off_outlined
                              : Icons.visibility_outlined,
                        ),
                      ),
                    ),
                    validator: (value) => value == null || value.isEmpty
                        ? 'Informe a senha.'
                        : null,
                    onChanged: (_) {
                      if (_hasError) setState(() => _hasError = false);
                    },
                    onFieldSubmitted: (_) => _submit(),
                  ),
                  if (_hasError)
                    Container(
                      margin: const EdgeInsets.only(top: 14),
                      padding: const EdgeInsets.all(12),
                      decoration: BoxDecoration(
                        color: Theme.of(context)
                            .colorScheme
                            .error
                            .withOpacity(0.09),
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: Row(
                        children: [
                          Icon(
                            Icons.error_outline,
                            size: 18,
                            color: Theme.of(context).colorScheme.error,
                          ),
                          const SizedBox(width: 9),
                          const Expanded(
                            child: Text('Usuário ou senha inválidos.'),
                          ),
                        ],
                      ),
                    ),
                ],
              ),
            ),
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: const Text('Cancelar'),
        ),
        ElevatedButton.icon(
          onPressed: _submit,
          icon: const Icon(Icons.login, size: 18),
          label: const Text('Entrar'),
        ),
      ],
    );
  }
}
