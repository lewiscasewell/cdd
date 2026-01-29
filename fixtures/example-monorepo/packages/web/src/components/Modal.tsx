// CIRCULAR (3-way): Modal → Form → Button → Modal
import React from 'react';
import { Form } from './Form';

interface ModalProps {
  children: React.ReactNode;
  onClose: () => void;
  showForm?: boolean;
}

export const Modal: React.FC<ModalProps> = ({ children, onClose, showForm }) => {
  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" onClick={(e) => e.stopPropagation()}>
        <button className="modal-close" onClick={onClose}>×</button>
        {showForm ? <Form onSubmit={onClose} /> : children}
      </div>
    </div>
  );
};
