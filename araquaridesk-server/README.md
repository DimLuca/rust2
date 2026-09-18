# AraquariDesk API

Serviço independente para autenticação dos técnicos, administração de usuários e auditoria do AraquariDesk. O RustDesk continua responsável somente pela comunicação remota.

## Segurança implementada

- senhas armazenadas com Argon2id;
- tokens JWT de curta duração, ligados a sessões persistidas e revogáveis;
- revogação de todas as sessões após redefinir senha ou desativar usuário;
- papéis `ADMIN` e `TECHNICIAN`;
- limitação de tentativas de login por IP e usuário;
- IP observado definido pelo servidor;
- `X-Forwarded-For` aceito somente de proxies presentes em `ARAQUARIDESK_TRUSTED_PROXY_CIDRS`;
- auditoria append-only: a aplicação pode inserir, mas não alterar ou excluir eventos;
- nenhuma senha, token, senha temporária RustDesk, tela, clipboard ou conteúdo de sessão é armazenado.

## Implantação com Docker

1. Entre na pasta `araquaridesk-server`.
2. Copie `.env.example` para `.env`.
3. Defina `POSTGRES_PASSWORD` no `.env`.
4. Gere um segredo aleatório para `ARAQUARIDESK_JWT_SECRET`, por exemplo com `openssl rand -base64 48`.
5. Configure os CIDRs dos proxies autorizados.
6. Inicie os serviços:

```bash
docker compose up -d --build
```

A API fica disponível somente em `127.0.0.1:8787` no host. O HTTPS deve terminar no proxy reverso institucional.

## Reverse proxy HTTPS

Exemplo de localização Nginx para `https://jiraiya.araquari.sc.gov.br/araquaridesk-api`:

```nginx
location /araquaridesk-api/ {
    proxy_pass http://127.0.0.1:8787/;
    proxy_http_version 1.1;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto https;
}
```

O endereço do proxy deve estar em `ARAQUARIDESK_TRUSTED_PROXY_CIDRS`. Não exponha diretamente a porta do PostgreSQL.

## Primeiro administrador

O utilitário solicita a senha sem exibi-la nem colocá-la no histórico do shell:

```bash
docker compose exec api araquaridesk-admin create-user \
  --username admin.exemplo \
  --display-name "Administrador de Exemplo" \
  --role admin
```

Não existe usuário ou senha padrão. O primeiro administrador passa a existir somente no PostgreSQL.

## Criar um técnico

```bash
docker compose exec api araquaridesk-admin create-user \
  --username tecnico.exemplo \
  --display-name "Técnico de Exemplo" \
  --role technician
```

## Alterar senha

```bash
docker compose exec api araquaridesk-admin reset-password \
  --username tecnico.exemplo
```

A senha antiga deixa de funcionar e todas as sessões anteriores são revogadas. Não é necessário recompilar o aplicativo.

## Desativar ou reativar usuário

```bash
docker compose exec api araquaridesk-admin disable-user --username tecnico.exemplo
docker compose exec api araquaridesk-admin enable-user --username tecnico.exemplo
```

## Consultar auditoria

Pelo servidor, sem expor token no terminal:

```bash
docker compose exec api araquaridesk-admin list-audit --limit 100
```

Usuários `ADMIN` também podem consultar `GET /api/v1/audit/events`. Usuários `TECHNICIAN` recebem `403` nesse endpoint.

## Configurar o cliente

Cadastre no GitHub em **Settings → Secrets and variables → Actions → Variables**:

```text
ARAQUARIDESK_API_URL=https://jiraiya.araquari.sc.gov.br/araquaridesk-api
```

Essa configuração é apenas um endereço público, não um segredo. O cliente possui o mesmo endereço como padrão institucional e não permite que o usuário comum o altere.

## Endpoints

| Método | Caminho | Autorização |
|---|---|---|
| `POST` | `/api/v1/auth/login` | público, limitado |
| `GET` | `/api/v1/auth/me` | sessão válida |
| `POST` | `/api/v1/auth/logout` | sessão válida |
| `POST` | `/api/v1/audit/events` | sessão válida |
| `GET` | `/api/v1/audit/events` | `ADMIN` |
| `GET/POST` | `/api/v1/users` | `ADMIN` |
| `PATCH` | `/api/v1/users/:id` | `ADMIN` |
| `POST` | `/api/v1/users/:id/reset-password` | `ADMIN` |
| `POST` | `/api/v1/users/:id/disable` | `ADMIN` |

## Migrations e logs

As migrations em `migrations/` são aplicadas automaticamente na inicialização. Para acompanhar os logs:

```bash
docker compose logs -f api
```

Os logs não incluem senhas nem tokens. Os timestamps do banco usam `TIMESTAMPTZ` e são definidos pelo servidor.

