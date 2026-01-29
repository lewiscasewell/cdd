// Non-circular: Controller → multiple services (tree)
import { OrderService } from '../services/orderService';
import { UserService } from '../services/userService';

export class OrderController {
  constructor(
    private orderService: OrderService,
    private userService: UserService
  ) {}

  async createOrder(userId: string, items: string[]) {
    const user = await this.userService.findById(userId);
    if (!user) throw new Error('User not found');
    return this.orderService.create(userId, items);
  }
}
