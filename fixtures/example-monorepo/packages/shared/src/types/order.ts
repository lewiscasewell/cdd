// Non-circular: type with dependency on user type
import { User } from './user';

export interface Order {
  id: string;
  user: User;
  items: OrderItem[];
  total: number;
  status: OrderStatus;
}

export interface OrderItem {
  productId: string;
  quantity: number;
  price: number;
}

export type OrderStatus = 'pending' | 'processing' | 'shipped' | 'delivered';
