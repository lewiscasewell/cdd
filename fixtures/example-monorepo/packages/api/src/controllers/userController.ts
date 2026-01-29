// Non-circular: Controller → Service (linear)
import { UserService } from '../services/userService';

export class UserController {
  constructor(private userService: UserService) {}

  async getUser(id: string) {
    return this.userService.findById(id);
  }

  async createUser(data: { name: string; email: string }) {
    return this.userService.create(data);
  }
}
