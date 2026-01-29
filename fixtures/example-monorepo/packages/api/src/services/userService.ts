// CIRCULAR: userService → orderService → userService
import { User } from '../models/user';
import { OrderService } from './orderService';

export class UserService {
  constructor(private orderService: OrderService) {}

  async findById(id: string): Promise<User | null> {
    // Simulated DB lookup
    return { id, name: 'John', email: 'john@example.com' };
  }

  async create(data: { name: string; email: string }): Promise<User> {
    return { id: 'new-id', ...data };
  }

  async getUserWithOrders(id: string) {
    const user = await this.findById(id);
    if (!user) return null;
    // This creates a circular dependency
    const orders = await this.orderService.findByUserId(id);
    return { ...user, orders };
  }
}
