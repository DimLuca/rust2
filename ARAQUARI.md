# Personalização institucional de Araquari

Esta distribuição mantém a base técnica do RustDesk e aplica a configuração da
Prefeitura na inicialização do cliente.

## Configuração incorporada

- servidor de ID: `jiraiya.araquari.sc.gov.br`;
- servidor relay: `jiraiya.araquari.sc.gov.br`;
- chave pública do servidor: incorporada em `src/araquari.rs`;
- cor principal: azul institucional `#27457E`;
- logos para temas claro e escuro e ícones dos pacotes: derivados da identidade
  visual fornecida pela Prefeitura, sem os elementos vermelhos.

O mesmo domínio atende os usuários internos e externos por DNS dividido:

- DNS interno: `jiraiya.araquari.sc.gov.br` deve apontar para `192.168.0.55`;
- DNS público: o mesmo nome deve apontar para o IP público que publica o
  RustDesk Server.

## Geração dos aplicativos

O workflow `Flutter Tag Build` já existente gera os pacotes do projeto. Para
produzir uma versão, crie uma tag no formato `vX.Y.Z` ou execute o workflow
manualmente em **Actions**. Os instaladores ficam nos artefatos e na release
correspondente.

Windows, Linux e macOS exigem runners diferentes; por isso a validação completa
dos instaladores ocorre no GitHub Actions, não em uma única máquina local.

## Acesso restrito da TI

O aplicativo sempre inicia no modo Usuário. O modo TI libera as ferramentas de
conexão de saída e as configurações somente após autenticação na AraquariDesk
API. Não existe usuário, senha ou hash administrativo no Flutter ou no GitHub.

Configure a variável de repositório `ARAQUARIDESK_API_URL` com o endpoint HTTPS.
As instruções de PostgreSQL, criação de administradores e técnicos, alteração de
senha, auditoria e reverse proxy estão em `araquaridesk-server/README.md`.

No modo Usuário, a janela é compacta e mostra somente o painel de recebimento de
suporte. Após o login da TI, a mesma janela expande e apresenta o painel de
controle remoto. Sair da TI revoga a sessão e retorna imediatamente ao layout
compacto.

## Assinatura do Windows

O pipeline aceita um certificado Authenticode por meio dos segredos
`WINDOWS_CODESIGN_PFX_BASE64` e `WINDOWS_CODESIGN_PFX_PASSWORD`. A URL do
servidor de timestamp pode ser definida na variável
`WINDOWS_CODESIGN_TIMESTAMP_URL`. A chave privada nunca deve ser adicionada ao
repositório.

Sem certificado, o workflow informa claramente que os pacotes estão sem
assinatura. Nenhuma proteção do Windows é desativada. A remoção consistente dos
alertas do SmartScreen depende de certificado confiável, timestamp e reputação
do editor.
