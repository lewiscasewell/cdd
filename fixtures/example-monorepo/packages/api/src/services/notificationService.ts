// Non-circular: standalone service
import { User } from '../models/user';

export class NotificationService {
  async sendEmail(user: User, subject: string, body: string) {
    console.log(`Sending email to ${user.email}: ${subject}`);
  }

  async sendPush(userId: string, message: string) {
    console.log(`Push to ${userId}: ${message}`);
  }
}
