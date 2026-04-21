import { createClient } from "@connectrpc/connect";
import { createGrpcWebTransport } from "@connectrpc/connect-web";
import { ArxService } from "@/src/gen/arx_connect";
import { CreateTenantRequest, CreateUserRequest } from "@/src/gen/arx_pb";

export class AdminService {
  private client;

  constructor(
    baseUrl: string,
    private adminKey: string,
  ) {
    const transport = createGrpcWebTransport({ baseUrl });
    this.client = createClient(ArxService, transport);
  }

  private headers() {
    return { Authorization: `Bearer ${this.adminKey}` };
  }

  async createTenant(name: string): Promise<string> {
    const res = await this.client.createTenant(new CreateTenantRequest({ name }), {
      headers: this.headers(),
    });
    if (res.error) throw new Error(res.error);
    return res.tenantId;
  }

  async createUser(tenantId: string, email: string, password: string): Promise<void> {
    const res = await this.client.createUser(
      new CreateUserRequest({ tenantId, email, password }),
      { headers: this.headers() },
    );
    if (res.error) throw new Error(res.error);
  }
}
