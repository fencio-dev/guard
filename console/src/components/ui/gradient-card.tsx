import * as React from "react"
import { cva, type VariantProps } from "class-variance-authority"

import { cn } from "@/lib/utils"
import { Card, CardHeader, CardFooter, CardTitle, CardDescription, CardContent } from "./card"

const gradientCardVariants = cva(
  "",
  {
    variants: {
      variant: {
        default: "",
        gradient: "relative overflow-hidden before:absolute before:inset-0 before:rounded-xl before:p-[1px] before:bg-gradient-to-b before:from-white/10 before:to-transparent before:-z-10 hover:before:from-white/20",
        glass: "bg-card/50 backdrop-blur-sm border-white/10",
      },
    },
    defaultVariants: {
      variant: "default",
    },
  }
)

export interface GradientCardProps
  extends React.HTMLAttributes<HTMLDivElement>,
    VariantProps<typeof gradientCardVariants> {
  hoverable?: boolean
}

const GradientCard = React.forwardRef<HTMLDivElement, GradientCardProps>(
  ({ className, variant, hoverable, ...props }, ref) => (
    <Card
      ref={ref}
      className={cn(gradientCardVariants({ variant }), className)}
      hoverable={hoverable}
      {...props}
    />
  )
)
GradientCard.displayName = "GradientCard"

export { GradientCard, CardHeader, CardFooter, CardTitle, CardDescription, CardContent }
