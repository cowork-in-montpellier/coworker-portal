import type { ReactNode } from 'react'
import { Link } from 'react-router-dom'

interface PageHeaderProps {
  title: ReactNode
  /** A short line under the title. */
  lede?: ReactNode
  /** Where "Retour" leads, shown at the top right. */
  back?: { to: string; label?: string }
  /** The page's main actions, centered under the title. */
  actions?: ReactNode
  className?: string
}

/** The Cowork'in page header: a centered Museo title over the blue rule. */
export function PageHeader({ title, lede, back, actions, className = '' }: PageHeaderProps) {
  return (
    <header className={`relative flex flex-col items-center gap-4 text-center ${className}`}>
      {back && (
        <Link to={back.to} className="btn btn-ghost btn-sm self-end sm:absolute sm:right-0 sm:top-0">
          ← {back.label ?? 'Retour'}
        </Link>
      )}
      <h1 className="cw-title sm:px-28">{title}</h1>
      <div className="cw-rule" aria-hidden="true" />
      {lede && <p className="font-bold text-base-content/75">{lede}</p>}
      {actions && <div className="flex w-full flex-wrap justify-center gap-2 sm:w-auto">{actions}</div>}
    </header>
  )
}
