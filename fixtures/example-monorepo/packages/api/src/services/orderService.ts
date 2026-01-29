// CIRCULAR: orderService → userService → orderService
import { Order } from '../models/order';
import { UserService } from './userService';

export class OrderService {
  constructor(private userService: UserService) {}

  async create(userId: string, items: string[]): Promise<Order> {
    return { id: 'order-1', userId, items, status: 'pending' };
  }

  async findByUserId(userId: string): Promise<Order[]> {
    return [{ id: 'order-1', userId, items: ['item-1'], status: 'completed' }];
  }

  async getOrderWithUser(orderId: string) {
    const order = await this.findById(orderId);
    if (!order) return null;
    // This creates a circular dependency back to userService
    const user = await this.userService.findById(order.userId);
    return { ...order, user };
  }

  async findById(id: string): Promise<Order | null> {
    return { id, userId: 'user-1', items: [], status: 'pending' };
  }
}
