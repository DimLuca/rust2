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
conexão de saída e as configurações somente após autenticação local. Os dados de
acesso não ficam no código-fonte: configure os segredos de Actions
`ARAQUARI_TI_USERNAME_SHA256` e `ARAQUARI_TI_PASSWORD_SHA256` com os respectivos
resumos SHA-256 antes de gerar o instalador Windows.
