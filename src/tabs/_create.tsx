import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import AppForm from "@/tabs/components/app-form";

const CreateTab = () => {
  return (
    <Card className="w-full">
      <CardHeader>
        <CardTitle>Create a Backup</CardTitle>
        <CardDescription>
          Select the applications you want to back up.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <AppForm />
      </CardContent>
    </Card>
  );
};

export { CreateTab };
