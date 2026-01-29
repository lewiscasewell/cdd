// CIRCULAR (3-way): Form → Button → Modal → Form
import React from 'react';
import { Button } from './Button';

interface FormProps {
  onSubmit: () => void;
}

export const Form: React.FC<FormProps> = ({ onSubmit }) => {
  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSubmit();
  };

  return (
    <form onSubmit={handleSubmit}>
      <input type="text" placeholder="Name" />
      <input type="email" placeholder="Email" />
      <Button variant="primary">Submit</Button>
    </form>
  );
};
